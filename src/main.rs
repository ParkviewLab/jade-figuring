// SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! jade-figuring: a calculator MCP server.

mod server;
mod tools;

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, Instant};

/// The version, from VERSION.txt at build time.
pub const VERSION: &str = env!("JADE_FIGURING_VERSION");

fn port() -> u16 {
    std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(35833)
}

/// `jade-figuring health`: exits 0 if this host's server answers `/health`,
/// for the container's health check (the image has no curl).
fn health_check() -> std::process::ExitCode {
    let ok = (|| -> std::io::Result<bool> {
        let addr = SocketAddr::from(([127, 0, 0, 1], port()));
        let mut s = TcpStream::connect_timeout(&addr, Duration::from_secs(3))?;
        s.set_read_timeout(Some(Duration::from_secs(3)))?;
        s.write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")?;
        let mut body = String::new();
        s.read_to_string(&mut body)?;
        Ok(body.starts_with("HTTP/1.1 200")
            && body.contains("\"ok\":true")
            && body.contains("uptime_seconds"))
    })()
    .unwrap_or(false);
    if ok {
        std::process::ExitCode::SUCCESS
    } else {
        std::process::ExitCode::FAILURE
    }
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    match std::env::args().nth(1).as_deref() {
        Some("health") => return health_check(),
        Some("--version" | "version") => {
            println!("jade-figuring {VERSION}");
            return std::process::ExitCode::SUCCESS;
        }
        Some(other) => {
            eprintln!(
                "unknown argument '{other}'; run with no arguments to serve, or `health` to check a running server"
            );
            return std::process::ExitCode::FAILURE;
        }
        None => {}
    }
    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let addr: SocketAddr = format!("{host}:{}", port())
        .parse()
        .expect("HOST and PORT form an address");
    let settings = server::Settings::from_env();
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("the address can be bound");
    eprintln!("jade-figuring {VERSION} listening on http://{addr}/mcp");
    axum::serve(listener, server::app(&settings, Instant::now()))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
        .expect("the server runs");
    std::process::ExitCode::SUCCESS
}
