# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Redlib is a privacy-focused Reddit frontend written in Rust. It proxies all requests through its servers, uses OAuth token spoofing to access Reddit's API (emulating the Android app), and serves pages with zero JavaScript, no ads, and strict Content Security Policy headers.

## Build and Development Commands

```bash
cargo run                     # Start dev server on localhost:8080
cargo build --release         # Production build
cargo test                    # Run all tests
cargo test <test_name>        # Run a specific test
```

The release build uses LTO and strips symbols. Git commit hash is captured at compile time via `build.rs`.

## Architecture

### Request Flow

1. `main.rs` registers routes and starts Hyper server with security headers
2. `server.rs` handles routing and compression (gzip/brotli)
3. Route handlers (subreddit.rs, post.rs, user.rs, etc.) extract params and call Reddit API
4. `client.rs` makes OAuth-authenticated requests using tokens from `oauth.rs`
5. `utils.rs` transforms Reddit JSON into Redlib data structures (Post, Comment, User)
6. Askama templates render HTML responses

### Key Modules

| Module | Purpose |
|--------|---------|
| `main.rs` | Entry point, route registration, CLI args (-p port, -a address, -H HSTS, -4/-6 IP version) |
| `server.rs` | HTTP server, routing via route_recognizer, request/response middleware |
| `client.rs` | HTTP client with Rustls TLS, OAuth token management, media proxying |
| `oauth.rs` | OAuth 2.0 auth, token refresh (24hr), Android client spoofing |
| `utils.rs` | Data structures, JSON parsing, URL formatting, template helpers |
| `config.rs` | Config from env vars (REDLIB_*), TOML file, or CLI |
| `settings.rs` | User preferences stored in cookies (theme, layout, sort, etc.) |

### Route Handlers

- `/r/:sub` → `subreddit.rs::community()`
- `/comments/:id` → `post.rs`
- `/user/:name` → `user.rs`
- `/search` → `search.rs`
- `/c/:collection` → Multi-subreddit collections
- `/settings` → User preferences form

### Templates

Located in `/templates/`. Uses Askama templating. Template structs follow `*Template` naming convention.

## Code Patterns

- **LazyLock** for global singletons (CONFIG, INSTANCE_INFO, OAUTH_CLIENT)
- **Async/await** throughout with Tokio runtime
- Route handlers return `BoxResponse` (boxed futures for trait object flexibility)
- `#[cached]` macro for expensive operations
- `dbg_msg!` macro for debug output
- `#![forbid(unsafe_code)]` - no unsafe code allowed

## Configuration

Three sources (in priority order):
1. CLI arguments
2. Environment variables (`REDLIB_SFW_ONLY`, `REDLIB_BANNER`, etc.)
3. `redlib.toml` file

Key settings: SFW_ONLY, BANNER, ROBOTS_DISABLE_INDEXING, ENABLE_RSS, COLLECTIONS

## Security Features

- Strict CSP blocking external resources
- HSTS headers
- X-Frame-Options: DENY
- Firefox-like TLS cipher suites to avoid fingerprinting
- AI bot user-agent filtering (120+ blocked patterns)
- All media proxied through `/img/`, `/vid/`, `/thumb/` endpoints

## Testing

Tests are inline with implementation (`#[cfg(test)] mod tests`). Key test locations:
- `utils.rs` - Flair parsing, time formatting, URL handling
- `config.rs` - Configuration and env var parsing
- `collections.rs` - Collection validation
- `oauth.rs` - OAuth response handling

## Static Assets

- `/static/` - CSS, themes (13 total, catppuccinMocha default), fonts, minimal JS
- Themes defined in static CSS files
