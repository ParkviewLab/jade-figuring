// SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! jade-figuring: a calculator MCP server.

use std::net::SocketAddr;
use std::time::Instant;

use axum::{Json, Router, routing::get};
use serde_json::{Value, json};

/// The version, from VERSION.txt at build time.
pub const VERSION: &str = env!("JADE_FIGURING_VERSION");

fn app(started: Instant) -> Router {
    Router::new().route("/health", get(move || async move { Json(health(started)) }))
}

fn health(started: Instant) -> Value {
    json!({"ok": true, "version": VERSION, "uptime_seconds": started.elapsed().as_secs()})
}

#[tokio::main]
async fn main() {
    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(35833);
    let addr: SocketAddr = format!("{host}:{port}")
        .parse()
        .expect("HOST and PORT form an address");
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("the address can be bound");
    eprintln!("jade-figuring {VERSION} listening on http://{addr}");
    axum::serve(listener, app(Instant::now()))
        .await
        .expect("the server runs");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_reports_the_version() {
        let h = health(Instant::now());
        assert_eq!(h["ok"], true);
        assert_eq!(h["version"], VERSION);
    }
}
