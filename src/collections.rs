use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use crate::config;

/// Parsed collection listing exposed via the `/c/<name>` routes.
pub static COLLECTIONS: LazyLock<HashMap<String, String>> = LazyLock::new(|| parse_collection_map(config::get_setting("REDLIB_COLLECTIONS")));

/// Reddit's multireddit URL endpoint caps at ~100 subs. Stay under that.
const HOME_FEED_SAMPLE_SIZE: usize = 100;

/// Window length for the home-from-collections rotation. Within a window the
/// generated multireddit URL is identical, so the existing JSON cache hits.
const HOME_FEED_WINDOW_SECS: u64 = 600;

/// Build a deterministic `(window_id, "sub1+sub2+...")` for the
/// home-from-collections feed. Same `(window_id, collection-contents)` always
/// produces the same output, so consecutive requests in the same window share
/// a cache entry. Returns `None` when no collections are configured.
///
/// When `pinned` is `Some`, that window_id is used directly — pagination
/// links carry the current window forward so a deep rabbit-hole survives the
/// 10-minute boundary. When `None`, the window is derived from `now_unix`.
///
/// Uses a local `fastrand::Rng` instance — the global RNG is untouched.
pub fn sample_home_for_window(pinned: Option<u64>, now_unix: u64) -> Option<(u64, String)> {
	let window_id = pinned.unwrap_or(now_unix / HOME_FEED_WINDOW_SECS);
	let mut all = home_subs_unique();
	if all.is_empty() {
		return None;
	}
	let mut rng = fastrand::Rng::with_seed(window_id);
	rng.shuffle(&mut all);
	all.truncate(HOME_FEED_SAMPLE_SIZE);
	Some((window_id, all.join("+")))
}

/// Represents an individual collection entry for template rendering.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Collection {
	pub name: String,
	pub target: String,
}

/// Returns a sorted list of all configured collections.
pub fn all() -> Vec<Collection> {
	let mut entries: Vec<_> = COLLECTIONS
		.iter()
		.map(|(name, target)| Collection {
			name: name.to_string(),
			target: target.to_string(),
		})
		.collect();
	entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
	entries
}

/// Lookup the underlying multireddit string for a collection alias.
pub fn resolve(name: &str) -> Option<String> {
	COLLECTIONS.get(name).cloned()
}

/// Whether any collections are configured.
pub fn is_empty() -> bool {
	COLLECTIONS.is_empty()
}

/// Collection names excluded from the home-from-collections sample, parsed
/// from `REDLIB_HOME_EXCLUDED_COLLECTIONS` (`+`-separated, whitespace tolerant,
/// empty entries ignored). A sub is excluded from the home sample only if
/// every collection it appears in is on this list — cross-listed subs survive.
static HOME_EXCLUDED_COLLECTIONS: LazyLock<HashSet<String>> = LazyLock::new(|| {
	config::get_setting("REDLIB_HOME_EXCLUDED_COLLECTIONS")
		.unwrap_or_default()
		.split('+')
		.map(str::trim)
		.filter(|s| !s.is_empty())
		.map(String::from)
		.collect()
});

fn collect_subs_unique<F>(should_include_collection: F) -> Vec<String>
where
	F: Fn(&str) -> bool,
{
	let mut seen = std::collections::BTreeSet::new();
	for (name, target) in COLLECTIONS.iter() {
		if !should_include_collection(name) {
			continue;
		}
		for entry in target.split('+') {
			let trimmed = entry.trim();
			if trimmed.is_empty() {
				continue;
			}
			let name = trimmed.strip_prefix("r/").unwrap_or(trimmed);
			if name.is_empty() {
				continue;
			}
			seen.insert(name.to_string());
		}
	}
	seen.into_iter().collect()
}

/// Cached unique-subs list across every collection. Used by callers that
/// want the full deduplicated view (not the home-page sample).
static ALL_SUBS_UNIQUE: LazyLock<Vec<String>> = LazyLock::new(|| collect_subs_unique(|_| true));

/// Cached unique-subs list with `REDLIB_HOME_EXCLUDED_COLLECTIONS` applied.
/// Built once at first use because both COLLECTIONS and the exclude set are
/// derived from immutable env/config state.
static HOME_SUBS_UNIQUE: LazyLock<Vec<String>> = LazyLock::new(|| collect_subs_unique(|name| !HOME_EXCLUDED_COLLECTIONS.contains(name)));

/// Returns the deduplicated set of subreddit names across every configured
/// collection, with any `r/` prefix stripped. Names are sorted for stable
/// output. The underlying set is built once at first use.
pub fn all_subs_unique() -> Vec<String> {
	ALL_SUBS_UNIQUE.clone()
}

/// Like `all_subs_unique`, but skips collections listed in
/// `REDLIB_HOME_EXCLUDED_COLLECTIONS`. Used to build the home-page sample.
pub fn home_subs_unique() -> Vec<String> {
	HOME_SUBS_UNIQUE.clone()
}

fn parse_collection_map(value: Option<String>) -> HashMap<String, String> {
	let mut map = HashMap::new();
	let Some(value) = value else {
		return map;
	};

	for entry in value.split(';') {
		let trimmed = entry.trim();
		if trimmed.is_empty() {
			continue;
		}

		let Some((alias, subs)) = trimmed.split_once('=') else {
			continue;
		};

		let alias = alias.trim();
		let subs = subs.trim();

		if alias.is_empty() || subs.is_empty() {
			continue;
		}

		map.insert(alias.to_string(), subs.to_string());
	}

	map
}

#[cfg(test)]
mod tests {
	use super::*;
	use sealed_test::prelude::*;
	use std::collections::HashMap;

	#[test]
	fn parses_collections() {
		let map = parse_collection_map(Some("ai=singularity+claude;news = worldnews+technology".into()));
		assert_eq!(map.get("ai"), Some(&"singularity+claude".to_string()));
		assert_eq!(map.get("news"), Some(&"worldnews+technology".to_string()));
	}

	#[test]
	fn ignores_invalid_entries() {
		let map = parse_collection_map(Some("=xyz;foo=;bar".into()));
		assert_eq!(map, HashMap::new());
	}

	#[test]
	#[sealed_test]
	fn sample_home_empty_collections_returns_none() {
		assert!(sample_home_for_window(None, 1_500_000_000).is_none());
	}

	#[test]
	#[sealed_test(env = [("REDLIB_COLLECTIONS", "tech=rust+golang+python+ruby+kotlin")])]
	fn sample_home_deterministic_within_window() {
		// Two timestamps in the same 600s window must produce the same output.
		let a = sample_home_for_window(None, 1_500_000_000).expect("collections configured");
		let b = sample_home_for_window(None, 1_500_000_000 + 599).expect("collections configured");
		assert_eq!(a, b, "same window must yield same (window_id, path)");
	}

	#[test]
	#[sealed_test(env = [("REDLIB_COLLECTIONS", "tech=rust+golang+python+ruby+kotlin")])]
	fn sample_home_window_id_matches_formula() {
		let now = 1_500_000_000_u64;
		let (window_id, _) = sample_home_for_window(None, now).expect("collections configured");
		assert_eq!(window_id, now / HOME_FEED_WINDOW_SECS);
	}

	#[test]
	#[sealed_test(env = [("REDLIB_COLLECTIONS", "tech=rust+golang+python+ruby+kotlin+java+swift+elixir+ocaml+haskell")])]
	fn sample_home_changes_between_windows() {
		// With 10 subs (3.6M permutations), adjacent seeds producing the same
		// shuffle is vanishingly unlikely. We assert the paths differ.
		let (_, a_path) = sample_home_for_window(None, 1_500_000_000).expect("collections configured");
		let (_, b_path) = sample_home_for_window(None, 1_500_000_000 + HOME_FEED_WINDOW_SECS).expect("collections configured");
		assert_ne!(a_path, b_path, "different windows must yield different paths");
	}

	#[test]
	#[sealed_test(env = [("REDLIB_COLLECTIONS", "big=s001+s002+s003+s004+s005+s006+s007+s008+s009+s010+s011+s012+s013+s014+s015+s016+s017+s018+s019+s020+s021+s022+s023+s024+s025+s026+s027+s028+s029+s030+s031+s032+s033+s034+s035+s036+s037+s038+s039+s040+s041+s042+s043+s044+s045+s046+s047+s048+s049+s050+s051+s052+s053+s054+s055+s056+s057+s058+s059+s060+s061+s062+s063+s064+s065+s066+s067+s068+s069+s070+s071+s072+s073+s074+s075+s076+s077+s078+s079+s080+s081+s082+s083+s084+s085+s086+s087+s088+s089+s090+s091+s092+s093+s094+s095+s096+s097+s098+s099+s100+s101+s102+s103+s104+s105")])]
	fn sample_home_truncates_to_cap() {
		let (_, path) = sample_home_for_window(None, 1_500_000_000).expect("collections configured");
		assert_eq!(path.split('+').count(), HOME_FEED_SAMPLE_SIZE);
	}

	#[test]
	#[sealed_test(env = [
		("REDLIB_COLLECTIONS", "ok=rust+golang;nsfw=onlyhere1+onlyhere2"),
		("REDLIB_HOME_EXCLUDED_COLLECTIONS", "nsfw"),
	])]
	fn sample_home_excludes_named_collections() {
		let (_, path) = sample_home_for_window(None, 1_500_000_000).expect("collections configured");
		let subs: Vec<&str> = path.split('+').collect();
		assert!(subs.contains(&"rust"));
		assert!(subs.contains(&"golang"));
		assert!(!subs.contains(&"onlyhere1"), "excluded-only sub leaked: {path}");
		assert!(!subs.contains(&"onlyhere2"), "excluded-only sub leaked: {path}");
	}

	#[test]
	#[sealed_test(env = [
		// `shared` appears in both an included and an excluded collection;
		// exclusion is per-collection, so `shared` should still be sampled.
		("REDLIB_COLLECTIONS", "ok=rust+shared;nsfw=shared+onlyhere"),
		("REDLIB_HOME_EXCLUDED_COLLECTIONS", "nsfw"),
	])]
	fn sample_home_keeps_subs_present_in_non_excluded_collection() {
		let (_, path) = sample_home_for_window(None, 1_500_000_000).expect("collections configured");
		let subs: Vec<&str> = path.split('+').collect();
		assert!(subs.contains(&"shared"), "cross-listed sub was wrongly dropped: {path}");
		assert!(!subs.contains(&"onlyhere"));
	}

	#[test]
	#[sealed_test(env = [
		("REDLIB_COLLECTIONS", "ok=rust+golang;nsfw=onlyhere"),
		("REDLIB_HOME_EXCLUDED_COLLECTIONS", "  nsfw  + + other "),
	])]
	fn sample_home_excluded_list_tolerates_whitespace_and_empty_entries() {
		let (_, path) = sample_home_for_window(None, 1_500_000_000).expect("collections configured");
		assert!(!path.contains("onlyhere"));
	}

	#[test]
	#[sealed_test(env = [("REDLIB_COLLECTIONS", "tech=rust+golang+python+ruby+kotlin")])]
	fn sample_home_pinned_overrides_clock() {
		// Pinned window_id stays stable across arbitrary `now` advances —
		// this is what lets a deep rabbit-hole survive the 10-minute boundary.
		let pin = 42_u64;
		let (wid_a, path_a) = sample_home_for_window(Some(pin), 1_500_000_000).expect("collections configured");
		let (wid_b, path_b) = sample_home_for_window(Some(pin), 1_500_000_000 + 10 * HOME_FEED_WINDOW_SECS).expect("collections configured");
		assert_eq!(wid_a, pin);
		assert_eq!(wid_b, pin);
		assert_eq!(path_a, path_b, "pinned window must produce stable sample across time");
	}

	#[test]
	#[sealed_test(env = [("REDLIB_COLLECTIONS", "tech=rust+golang+python+ruby+kotlin")])]
	fn sample_home_pinned_wins_over_time_derived() {
		let now = 1_500_000_000_u64;
		let natural = now / HOME_FEED_WINDOW_SECS;
		let (wid, _) = sample_home_for_window(Some(natural + 1), now).expect("collections configured");
		assert_eq!(wid, natural + 1);
	}

	#[test]
	#[sealed_test(env = [("REDLIB_COLLECTIONS", "tech=rust+golang")])]
	fn sample_home_none_falls_back_to_clock() {
		let now = 1_500_000_000_u64;
		let (wid, _) = sample_home_for_window(None, now).expect("collections configured");
		assert_eq!(wid, now / HOME_FEED_WINDOW_SECS);
	}
}
