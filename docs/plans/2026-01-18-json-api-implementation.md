# JSON API Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add `.js` suffix routes to all content pages, returning JSON instead of HTML for MCP consumption.

**Architecture:** Separate JSON routes calling shared data-fetching functions. New `json.rs` module provides response helpers and structs. Existing structs get `Serialize` derives.

**Tech Stack:** Rust, Serde, Hyper, existing Redlib patterns

---

## Task 1: Add Serialize to Comment struct

**Files:**
- Modify: `src/utils.rs:470-492`

**Step 1: Add Serialize derive to Comment**

The Comment struct currently has only `#[template(path = "comment.html")]`. Add `Serialize` alongside it.

```rust
#[derive(Serialize, Template)]
#[template(path = "comment.html")]
/// Comment with content, post, score and data/time that it was posted
pub struct Comment {
	pub id: String,
	pub kind: String,
	pub parent_id: String,
	pub parent_kind: String,
	pub post_link: String,
	pub post_author: String,
	pub body: String,
	pub author: Author,
	pub score: (String, String),
	pub rel_time: String,
	pub created: String,
	pub edited: (String, String),
	pub replies: Vec<Comment>,
	pub highlighted: bool,
	pub awards: Awards,
	pub collapsed: bool,
	pub is_filtered: bool,
	pub more_count: i64,
	#[serde(skip)]
	pub prefs: Preferences,
}
```

Note: `prefs` field is skipped from serialization since it's UI-specific and `Preferences` may not implement `Serialize` in a way we want to expose.

**Step 2: Verify it compiles**

Run: `cargo check`
Expected: Compiles without errors

**Step 3: Commit**

```bash
git add src/utils.rs
git commit -m "feat(json): add Serialize derive to Comment struct"
```

---

## Task 2: Add Serialize to Subreddit struct

**Files:**
- Modify: `src/utils.rs:598-611`

**Step 1: Add Serialize derive to Subreddit**

```rust
#[derive(Default, Serialize)]
/// Subreddit struct containing metadata about community
pub struct Subreddit {
	pub name: String,
	pub title: String,
	pub description: String,
	pub info: String,
	// pub moderators: Vec<String>,
	pub icon: String,
	pub members: (String, String),
	pub active: (String, String),
	pub wiki: bool,
	pub nsfw: bool,
}
```

**Step 2: Verify it compiles**

Run: `cargo check`
Expected: Compiles without errors

**Step 3: Commit**

```bash
git add src/utils.rs
git commit -m "feat(json): add Serialize derive to Subreddit struct"
```

---

## Task 3: Add Serialize to User struct

**Files:**
- Modify: `src/utils.rs:585-596`

**Step 1: Add Serialize derive to User**

```rust
#[derive(Default, Serialize)]
/// User struct containing metadata about user
pub struct User {
	pub name: String,
	pub title: String,
	pub icon: String,
	pub karma: i64,
	pub created: String,
	pub banner: String,
	pub description: String,
	pub nsfw: bool,
}
```

**Step 2: Verify it compiles**

Run: `cargo check`
Expected: Compiles without errors

**Step 3: Commit**

```bash
git add src/utils.rs
git commit -m "feat(json): add Serialize derive to User struct"
```

---

## Task 4: Create json.rs module with helpers

**Files:**
- Create: `src/json.rs`
- Modify: `src/main.rs` (add mod declaration)

**Step 1: Create the json.rs module**

```rust
//! JSON API response helpers and structs.

use hyper::{Body, Response};
use serde::Serialize;

use crate::utils::{Comment, Post, Subreddit, User};

/// Wrapper for all JSON API responses.
#[derive(Serialize)]
pub struct JsonResponse<T: Serialize> {
	pub data: Option<T>,
	pub error: Option<String>,
}

/// Build a successful JSON response.
pub fn json_response<T: Serialize>(data: T) -> Response<Body> {
	let response = JsonResponse {
		data: Some(data),
		error: None,
	};
	Response::builder()
		.status(200)
		.header("content-type", "application/json")
		.body(serde_json::to_string(&response).unwrap_or_default().into())
		.unwrap_or_default()
}

/// Build an error JSON response.
pub fn json_error(msg: String, status: u16) -> Response<Body> {
	let response: JsonResponse<()> = JsonResponse {
		data: None,
		error: Some(msg),
	};
	Response::builder()
		.status(status)
		.header("content-type", "application/json")
		.body(serde_json::to_string(&response).unwrap_or_default().into())
		.unwrap_or_default()
}

// --- Response structs for each endpoint ---

#[derive(Serialize)]
pub struct SubredditResponse {
	pub subreddit: Subreddit,
	pub posts: Vec<Post>,
	pub after: Option<String>,
}

#[derive(Serialize)]
pub struct PostResponse {
	pub post: Post,
	pub comments: Vec<Comment>,
}

#[derive(Serialize)]
pub struct UserResponse {
	pub user: User,
	pub posts: Vec<Post>,
	pub after: Option<String>,
}

#[derive(Serialize)]
pub struct SearchResponse {
	pub posts: Vec<Post>,
	pub after: Option<String>,
}

#[derive(Serialize)]
pub struct WikiResponse {
	pub subreddit: String,
	pub page: String,
	pub content: String,
}

#[derive(Serialize)]
pub struct DuplicatesResponse {
	pub post: Post,
	pub duplicates: Vec<Post>,
}
```

**Step 2: Add mod declaration to main.rs**

In `src/main.rs`, near the top with other mod declarations, add:

```rust
mod json;
```

**Step 3: Verify it compiles**

Run: `cargo check`
Expected: Compiles without errors

**Step 4: Commit**

```bash
git add src/json.rs src/main.rs
git commit -m "feat(json): add json module with response helpers and structs"
```

---

## Task 5: Add JSON endpoint for subreddit

**Files:**
- Modify: `src/subreddit.rs`
- Modify: `src/main.rs`

**Step 1: Add community_json handler to subreddit.rs**

At the top of `src/subreddit.rs`, add the import:

```rust
use crate::json::{json_error, json_response, SubredditResponse};
```

After the `community` function (around line 234), add:

```rust
/// JSON API endpoint for subreddit/community data.
pub async fn community_json(req: Request<Body>) -> Result<Response<Body>, String> {
	let query = req.uri().query().unwrap_or_default().to_string();
	let subscribed = setting(&req, "subscriptions");
	let front_page = setting(&req, "front_page");
	let post_sort = req.cookie("post_sort").map_or_else(|| "hot".to_string(), |c| c.value().to_string());
	let sort = req.param("sort").unwrap_or_else(|| req.param("id").unwrap_or(post_sort));
	let default_front = if front_page == "default" || front_page.is_empty() {
		if subscribed.is_empty() {
			"popular".to_string()
		} else {
			subscribed.clone()
		}
	} else {
		front_page.clone()
	};

	let collection_param = req.param("collection");
	let sub_name = if let Some(sub) = req.param("sub") {
		sub
	} else if let Some(alias) = collection_param.clone() {
		match collections::resolve(&alias) {
			Some(target) => target,
			None => return Ok(json_error(format!("Collection \"{alias}\" is not configured"), 404)),
		}
	} else {
		default_front
	};

	let quarantined = can_access_quarantine(&req, &sub_name);

	// Handle random subreddits - return error for JSON API
	if sub_name == "random" || sub_name == "randnsfw" {
		return Ok(json_error("Random subreddits not supported in JSON API".to_string(), 400));
	}

	// Request subreddit metadata
	let sub = if !sub_name.contains('+') && sub_name != subscribed && sub_name != "popular" && sub_name != "all" {
		subreddit(&sub_name, quarantined).await.unwrap_or_default()
	} else if sub_name == subscribed {
		Subreddit::default()
	} else {
		Subreddit {
			name: sub_name.clone(),
			..Subreddit::default()
		}
	};

	// Check NSFW gating (server-side SFW_ONLY only)
	if sub.nsfw && crate::utils::sfw_only() {
		return Ok(json_error("NSFW content is disabled on this instance".to_string(), 403));
	}

	let mut params = String::from("&raw_json=1");
	if sub_name == "popular" {
		let geo_filter = match GEO_FILTER_MATCH.captures(&query) {
			Some(geo_filter) => geo_filter["region"].to_string(),
			None => "GLOBAL".to_owned(),
		};
		params.push_str(&format!("&geo_filter={geo_filter}"));
	}

	let path = format!("/r/{}/{sort}.json?{}{params}", sub_name.replace('+', "%2B"), req.uri().query().unwrap_or_default());

	match Post::fetch(&path, quarantined).await {
		Ok((posts, after)) => {
			let response = SubredditResponse {
				subreddit: sub,
				posts,
				after: if after.is_empty() { None } else { Some(after) },
			};
			Ok(json_response(response))
		}
		Err(msg) => match msg.as_str() {
			"quarantined" | "gated" => Ok(json_error(format!("r/{sub_name} is {msg}"), 403)),
			"private" => Ok(json_error(format!("r/{sub_name} is a private community"), 403)),
			"banned" => Ok(json_error(format!("r/{sub_name} has been banned from Reddit"), 404)),
			_ => Ok(json_error(msg, 500)),
		},
	}
}
```

**Step 2: Register JSON routes in main.rs**

In `src/main.rs`, after the existing subreddit routes (around line 307-351), add:

```rust
// JSON API routes for subreddits
app.at("/r/:sub.js").get(|r| subreddit::community_json(r).boxed());
app.at("/r/:sub/:sort.js").get(|r| subreddit::community_json(r).boxed());
app.at("/c/:collection.js").get(|r| subreddit::community_json(r).boxed());
app.at("/c/:collection/:sort.js").get(|r| subreddit::community_json(r).boxed());
```

**Step 3: Test manually**

Run: `cargo run`
Then: `curl http://localhost:8080/r/rust.js | jq .`
Expected: JSON response with subreddit data and posts

**Step 4: Commit**

```bash
git add src/subreddit.rs src/main.rs
git commit -m "feat(json): add JSON API endpoint for subreddits"
```

---

## Task 6: Add JSON endpoint for posts

**Files:**
- Modify: `src/post.rs`
- Modify: `src/main.rs`

**Step 1: Add imports to post.rs**

At the top of `src/post.rs`, add:

```rust
use crate::json::{json_error, json_response, PostResponse};
```

**Step 2: Add item_json handler to post.rs**

After the `item` function (around line 113), add:

```rust
/// JSON API endpoint for post data.
pub async fn item_json(req: Request<Body>) -> Result<Response<Body>, String> {
	let path: String = format!("{}.json?{}&raw_json=1", req.uri().path().trim_end_matches(".js"), req.uri().query().unwrap_or_default());
	let sub = req.param("sub").unwrap_or_default();
	let quarantined = can_access_quarantine(&req, &sub);

	let sort = param(&path, "sort").unwrap_or_default();
	let highlighted_comment = &req.param("comment_id").unwrap_or_default();

	match json(path, quarantined).await {
		Ok(response) => {
			let post = parse_post(&response[0]["data"]["children"][0]).await;

			// Check NSFW gating (server-side SFW_ONLY only)
			if post.nsfw && crate::utils::sfw_only() {
				return Ok(json_error("NSFW content is disabled on this instance".to_string(), 403));
			}

			let filters = get_filters(&req);
			let comments = parse_comments(&response[1], &post.permalink, &post.author.name, highlighted_comment, &filters, &req);

			Ok(json_response(PostResponse { post, comments }))
		}
		Err(msg) => {
			if msg == "quarantined" || msg == "gated" {
				Ok(json_error(format!("Post is {msg}"), 403))
			} else {
				Ok(json_error(msg, 500))
			}
		}
	}
}
```

**Step 3: Register JSON routes in main.rs**

In `src/main.rs`, after the existing post routes (around line 324-331), add:

```rust
// JSON API routes for posts
app.at("/r/:sub/comments/:id.js").get(|r| post::item_json(r).boxed());
app.at("/r/:sub/comments/:id/:title.js").get(|r| post::item_json(r).boxed());
app.at("/r/:sub/comments/:id/:title/:comment_id.js").get(|r| post::item_json(r).boxed());
app.at("/comments/:id.js").get(|r| post::item_json(r).boxed());
app.at("/comments/:id/:title.js").get(|r| post::item_json(r).boxed());
app.at("/comments/:id/:title/:comment_id.js").get(|r| post::item_json(r).boxed());
```

**Step 4: Test manually**

Run: `cargo run`
Then: `curl "http://localhost:8080/r/rust/comments/abc123.js" | jq .`
Expected: JSON response with post and comments (or error if post doesn't exist)

**Step 5: Commit**

```bash
git add src/post.rs src/main.rs
git commit -m "feat(json): add JSON API endpoint for posts"
```

---

## Task 7: Add JSON endpoint for user profiles

**Files:**
- Modify: `src/user.rs`
- Modify: `src/main.rs`

**Step 1: Add imports to user.rs**

At the top of `src/user.rs`, add:

```rust
use crate::json::{json_error, json_response, UserResponse};
```

**Step 2: Add profile_json handler to user.rs**

After the `profile` function (around line 107), add:

```rust
/// JSON API endpoint for user profile data.
pub async fn profile_json(req: Request<Body>) -> Result<Response<Body>, String> {
	let listing = req.param("listing").unwrap_or_else(|| "overview".to_string());

	let path = format!(
		"/user/{}/{listing}.json?{}&raw_json=1",
		req.param("name").unwrap_or_else(|| "reddit".to_string()),
		req.uri().query().unwrap_or_default(),
	);

	let username = req.param("name").unwrap_or_default();
	let user = user(&username).await.unwrap_or_default();

	// Check NSFW gating (server-side SFW_ONLY only)
	if user.nsfw && crate::utils::sfw_only() {
		return Ok(json_error("NSFW content is disabled on this instance".to_string(), 403));
	}

	match Post::fetch(&path, false).await {
		Ok((posts, after)) => {
			let response = UserResponse {
				user,
				posts,
				after: if after.is_empty() { None } else { Some(after) },
			};
			Ok(json_response(response))
		}
		Err(msg) => Ok(json_error(msg, 500)),
	}
}
```

**Step 3: Register JSON routes in main.rs**

In `src/main.rs`, after the existing user routes (around line 291-295), add:

```rust
// JSON API routes for users
app.at("/user/:name.js").get(|r| user::profile_json(r).boxed());
app.at("/user/:name/:listing.js").get(|r| user::profile_json(r).boxed());
```

**Step 4: Test manually**

Run: `cargo run`
Then: `curl http://localhost:8080/user/spez.js | jq .`
Expected: JSON response with user data and posts

**Step 5: Commit**

```bash
git add src/user.rs src/main.rs
git commit -m "feat(json): add JSON API endpoint for user profiles"
```

---

## Task 8: Add JSON endpoint for search

**Files:**
- Modify: `src/search.rs`
- Modify: `src/main.rs`

**Step 1: Add imports to search.rs**

At the top of `src/search.rs`, add:

```rust
use crate::json::{json_error, json_response, SearchResponse};
```

**Step 2: Add find_json handler to search.rs**

After the `find` function, add:

```rust
/// JSON API endpoint for search results.
pub async fn find_json(req: Request<Body>) -> Result<Response<Body>, String> {
	let nsfw_results = if !utils::sfw_only() { "&include_over_18=on" } else { "" };
	let uri_path = req.uri().path().replace("+", "%2B").trim_end_matches(".js").to_string();
	let path = format!("{}.json?{}{}&raw_json=1", uri_path, req.uri().query().unwrap_or_default(), nsfw_results);
	let query = param(&path, "q").unwrap_or_default();

	if query.is_empty() {
		return Ok(json_error("Search query is required".to_string(), 400));
	}

	let sub = req.param("sub").unwrap_or_default();
	let quarantined = can_access_quarantine(&req, &sub);

	let typed = param(&path, "type").unwrap_or_default();

	// Only return posts for JSON API (not subreddit suggestions)
	if typed == "sr" {
		return Ok(json_error("Subreddit search not supported in JSON API, use post search".to_string(), 400));
	}

	match Post::fetch(&path, quarantined).await {
		Ok((posts, after)) => {
			let response = SearchResponse {
				posts,
				after: if after.is_empty() { None } else { Some(after) },
			};
			Ok(json_response(response))
		}
		Err(msg) => Ok(json_error(msg, 500)),
	}
}
```

**Step 3: Register JSON routes in main.rs**

In `src/main.rs`, after the existing search routes (around line 338, 365), add:

```rust
// JSON API routes for search
app.at("/search.js").get(|r| search::find_json(r).boxed());
app.at("/r/:sub/search.js").get(|r| search::find_json(r).boxed());
```

**Step 4: Test manually**

Run: `cargo run`
Then: `curl "http://localhost:8080/search.js?q=rust" | jq .`
Expected: JSON response with search results

**Step 5: Commit**

```bash
git add src/search.rs src/main.rs
git commit -m "feat(json): add JSON API endpoint for search"
```

---

## Task 9: Add JSON endpoint for wiki

**Files:**
- Modify: `src/subreddit.rs`
- Modify: `src/main.rs`

**Step 1: Add WikiResponse import**

Update the import in `src/subreddit.rs`:

```rust
use crate::json::{json_error, json_response, SubredditResponse, WikiResponse};
```

**Step 2: Add wiki_json handler to subreddit.rs**

After the `wiki` function (around line 526), add:

```rust
/// JSON API endpoint for wiki pages.
pub async fn wiki_json(req: Request<Body>) -> Result<Response<Body>, String> {
	let sub = req.param("sub").unwrap_or_else(|| "reddit.com".to_string());
	let quarantined = can_access_quarantine(&req, &sub);

	// Handle random subreddits - return error for JSON API
	if sub == "random" || sub == "randnsfw" {
		return Ok(json_error("Random subreddits not supported in JSON API".to_string(), 400));
	}

	let page = req.param("page").unwrap_or_else(|| "index".to_string());
	let path: String = format!("/r/{sub}/wiki/{page}.json?raw_json=1");

	match json(path, quarantined).await {
		Ok(response) => {
			let content = rewrite_urls(response["data"]["content_html"].as_str().unwrap_or(""));
			Ok(json_response(WikiResponse {
				subreddit: sub,
				page,
				content,
			}))
		}
		Err(msg) => {
			if msg == "quarantined" || msg == "gated" {
				Ok(json_error(format!("r/{sub} is {msg}"), 403))
			} else {
				Ok(json_error(msg, 500))
			}
		}
	}
}
```

**Step 3: Register JSON routes in main.rs**

In `src/main.rs`, after the existing wiki routes (around line 346-362), add:

```rust
// JSON API routes for wiki
app.at("/r/:sub/wiki.js").get(|r| subreddit::wiki_json(r).boxed());
app.at("/r/:sub/wiki/*page.js").get(|r| subreddit::wiki_json(r).boxed());
app.at("/wiki.js").get(|r| subreddit::wiki_json(r).boxed());
app.at("/wiki/*page.js").get(|r| subreddit::wiki_json(r).boxed());
```

**Step 4: Test manually**

Run: `cargo run`
Then: `curl http://localhost:8080/r/rust/wiki.js | jq .`
Expected: JSON response with wiki content

**Step 5: Commit**

```bash
git add src/subreddit.rs src/main.rs
git commit -m "feat(json): add JSON API endpoint for wiki pages"
```

---

## Task 10: Add JSON endpoint for duplicates

**Files:**
- Modify: `src/duplicates.rs`
- Modify: `src/main.rs`

**Step 1: Add imports to duplicates.rs**

At the top of `src/duplicates.rs`, add:

```rust
use crate::json::{json_error, json_response, DuplicatesResponse};
```

**Step 2: Add item_json handler to duplicates.rs**

After the `item` function, add:

```rust
/// JSON API endpoint for duplicate posts.
pub async fn item_json(req: Request<Body>) -> Result<Response<Body>, String> {
	let path: String = format!("{}.json?{}&raw_json=1", req.uri().path().trim_end_matches(".js"), req.uri().query().unwrap_or_default());
	let sub = req.param("sub").unwrap_or_default();
	let quarantined = can_access_quarantine(&req, &sub);

	match json(path, quarantined).await {
		Ok(response) => {
			let post = parse_post(&response[0]["data"]["children"][0]).await;

			// Check NSFW gating (server-side SFW_ONLY only)
			if post.nsfw && crate::utils::sfw_only() {
				return Ok(json_error("NSFW content is disabled on this instance".to_string(), 403));
			}

			let filters = get_filters(&req);
			let (duplicates, _, _) = parse_duplicates(&response[1], &filters).await;

			Ok(json_response(DuplicatesResponse { post, duplicates }))
		}
		Err(msg) => {
			if msg == "quarantined" || msg == "gated" {
				Ok(json_error(format!("Post is {msg}"), 403))
			} else {
				Ok(json_error(msg, 500))
			}
		}
	}
}
```

**Step 3: Register JSON routes in main.rs**

In `src/main.rs`, after the existing duplicates routes (around line 333-336), add:

```rust
// JSON API routes for duplicates
app.at("/r/:sub/duplicates/:id.js").get(|r| duplicates::item_json(r).boxed());
app.at("/r/:sub/duplicates/:id/:title.js").get(|r| duplicates::item_json(r).boxed());
app.at("/duplicates/:id.js").get(|r| duplicates::item_json(r).boxed());
app.at("/duplicates/:id/:title.js").get(|r| duplicates::item_json(r).boxed());
```

**Step 4: Test manually**

Run: `cargo run`
Then: `curl http://localhost:8080/duplicates/abc123.js | jq .`
Expected: JSON response with post and duplicates (or error)

**Step 5: Commit**

```bash
git add src/duplicates.rs src/main.rs
git commit -m "feat(json): add JSON API endpoint for duplicates"
```

---

## Task 11: Run full test suite

**Step 1: Run tests**

Run: `cargo test`
Expected: All tests pass

**Step 2: Run clippy**

Run: `cargo clippy`
Expected: No errors (warnings okay)

**Step 3: Build release**

Run: `cargo build --release`
Expected: Builds successfully

**Step 4: Commit any fixes if needed**

---

## Task 12: Final integration test

**Step 1: Start the server**

Run: `cargo run`

**Step 2: Test all JSON endpoints**

```bash
# Subreddit
curl -s "http://localhost:8080/r/rust.js" | jq '.data.subreddit.name'
# Expected: "rust"

# Post (use a real post ID)
curl -s "http://localhost:8080/r/rust/comments/abc123.js" | jq '.error'
# Expected: error message or post data

# User
curl -s "http://localhost:8080/user/spez.js" | jq '.data.user.name'
# Expected: "spez"

# Search
curl -s "http://localhost:8080/search.js?q=rust" | jq '.data.posts | length'
# Expected: number > 0

# Collection (if configured)
curl -s "http://localhost:8080/c/news.js" | jq '.error // .data.posts | length'
# Expected: error or number
```

**Step 3: Final commit**

```bash
git add -A
git commit -m "feat(json): complete JSON API implementation

Add .js suffix routes to all content pages:
- /r/:sub.js - subreddit posts
- /r/:sub/comments/:id.js - post with comments
- /user/:name.js - user profile
- /search.js - search results
- /r/:sub/wiki.js - wiki pages
- /duplicates/:id.js - duplicate posts
- /c/:collection.js - collections

All responses use consistent {data, error} envelope format.
NSFW filtering respects server-side SFW_ONLY config only."
```
