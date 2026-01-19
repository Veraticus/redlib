//! JSON API response helpers and structs.

use hyper::{Body, Response};
use serde::Serialize;

use crate::utils::{truncate_body, Comment, Post, Subreddit, User};

/// Default body truncation limit for list endpoints (search, subreddit, user, duplicates)
pub const DEFAULT_BODY_LIMIT: usize = 400;

/// Truncate body of a single post if body_limit is specified.
pub fn truncate_post(post: &mut Post, body_limit: Option<usize>) {
	if let Some(limit) = body_limit {
		let (truncated_body, was_truncated) = truncate_body(&post.body, limit);
		post.body = truncated_body;
		if was_truncated {
			post.body_truncated = Some(true);
		}
	}
}

/// Truncate bodies of multiple posts.
pub fn truncate_posts(posts: &mut [Post], body_limit: Option<usize>) {
	for post in posts.iter_mut() {
		truncate_post(post, body_limit);
	}
}

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
