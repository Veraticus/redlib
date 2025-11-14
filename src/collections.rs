use std::collections::HashMap;
use std::sync::LazyLock;

use crate::config;

/// Parsed collection listing exposed via the `/c/<name>` routes.
pub static COLLECTIONS: LazyLock<HashMap<String, String>> = LazyLock::new(|| parse_collection_map(config::get_setting("REDLIB_COLLECTIONS")));

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
	use super::parse_collection_map;
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
}
