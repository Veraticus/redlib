# JSON API Design

Add `.js` suffix support to all content pages, returning JSON instead of HTML.

## Use Case

MCP (Model Context Protocol) server consuming Redlib data programmatically.

## Scope

| Route | JSON Route | Returns |
|-------|------------|---------|
| `/r/:sub` | `/r/:sub.js` | Posts array, subreddit info, pagination cursor |
| `/r/:sub/comments/:id/:title` | `/r/:sub/comments/:id/:title.js` | Post, comments tree |
| `/user/:name` | `/user/:name.js` | User info, posts/comments |
| `/search` | `/search.js` | Search results |
| `/r/:sub/search` | `/r/:sub/search.js` | Subreddit search results |
| `/r/:sub/wiki/:page` | `/r/:sub/wiki/:page.js` | Wiki content |
| `/duplicates/:id` | `/duplicates/:id.js` | Cross-posts |
| `/c/:collection` | `/c/:collection.js` | Collection posts |

## Response Format

Simple envelope using Redlib's native structs:

```json
{
  "data": { ... },
  "error": null
}
```

On error:

```json
{
  "data": null,
  "error": "Subreddit not found"
}
```

Content-Type: `application/json`

## Struct Changes

**Add `#[derive(Serialize)]` to:**
- `Comment` in `utils.rs`
- `Subreddit` in `utils.rs`

**Already serializable (no changes):**
- `Post`, `Author`, `Flair`, `FlairPart`, `Media`, `GalleryMedia`, `Poll`, `PollOption`, `Flags`, `Awards`, `Award`

## New Module: `src/json.rs`

### Helper Functions

```rust
use serde::Serialize;
use hyper::{Body, Response};

#[derive(Serialize)]
pub struct JsonResponse<T: Serialize> {
    pub data: Option<T>,
    pub error: Option<String>,
}

pub fn json_response<T: Serialize>(data: T) -> Response<Body> {
    let response = JsonResponse { data: Some(data), error: None };
    Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(serde_json::to_string(&response).unwrap_or_default().into())
        .unwrap_or_default()
}

pub fn json_error(msg: String, status: u16) -> Response<Body> {
    let response: JsonResponse<()> = JsonResponse { data: None, error: Some(msg) };
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(serde_json::to_string(&response).unwrap_or_default().into())
        .unwrap_or_default()
}
```

### Response Structs

```rust
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
    pub comments: Vec<Comment>,
    pub after: Option<String>,
}

#[derive(Serialize)]
pub struct SearchResponse {
    pub results: Vec<Post>,
    pub after: Option<String>,
}
```

## Routing Pattern

Separate routes with shared data-fetching logic:

```rust
// main.rs
route("/r/:sub", subreddit::community)
route("/r/:sub.js", subreddit::community_json)
```

Handler pattern:

```rust
// Shared data fetching
async fn fetch_community_data(req: &Request<Body>) -> Result<SubredditResponse, String> {
    // existing fetch logic, returns structured data
}

// HTML endpoint (existing, refactored)
pub fn community(req: Request<Body>) -> BoxResponse {
    Box::pin(async move {
        match fetch_community_data(&req).await {
            Ok(data) => template(&SubredditTemplate { ... }),
            Err(e) => error_page(&req, e),
        }
    })
}

// JSON endpoint (new)
pub fn community_json(req: Request<Body>) -> BoxResponse {
    Box::pin(async move {
        match fetch_community_data(&req).await {
            Ok(data) => json_response(data),
            Err(e) => json_error(e, 500),
        }
    })
}
```

## Pagination

Include cursor in response for next page:

```rust
pub struct SubredditResponse {
    pub subreddit: Subreddit,
    pub posts: Vec<Post>,
    pub after: Option<String>,  // cursor for next page
}
```

Clients fetch next page: `/r/rust.js?after=t3_abc123`

Query parameters pass through: `?sort=`, `?after=`, `?t=`

## NSFW Handling

- Respect server-side `REDLIB_SFW_ONLY` config
- Ignore user cookie preferences
- MCP receives all content the instance allows

## Implementation Order

### Phase 1: Foundation
1. Add `#[derive(Serialize)]` to `Comment` and `Subreddit` in `utils.rs`
2. Create `src/json.rs` with helpers and response structs
3. Add `mod json;` to `main.rs`

### Phase 2: Proof of Concept
1. Refactor `subreddit.rs` — extract data fetching
2. Add `community_json` handler
3. Register `/r/:sub.js` route
4. Test

### Phase 3: Remaining Endpoints
1. Posts (`/r/:sub/comments/:id/:title.js`)
2. User profiles (`/user/:name.js`)
3. Search (`/search.js`, `/r/:sub/search.js`)
4. Wiki (`/r/:sub/wiki/:page.js`)
5. Duplicates (`/duplicates/:id.js`)
6. Collections (`/c/:collection.js`)

### Phase 4: Edge Cases
1. Error handling consistency (404s, Reddit API errors)
2. Query parameter passthrough
3. Verify NSFW/SFW filtering applies
