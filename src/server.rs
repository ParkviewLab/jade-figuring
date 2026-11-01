// SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The MCP server (stateless streamable HTTP at `/mcp`, JSON responses) and
//! the `/health` and `/admin/version` routes.

use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::{Json, Router, routing::get};
use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    model::{
        CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation,
        ListToolsResult, PaginatedRequestParams, ServerCapabilities, ServerConfig, Tool,
    },
    service::RequestContext,
    transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
    },
};
use serde_json::{Value, json};

use crate::tools;
use jade_figuring::{calc, dates};

/// A tool call is abandoned after this long, beyond the calculators' own limit.
const CALL_LIMIT: Duration = Duration::from_secs(3);

#[derive(Clone)]
pub struct Calculators;

impl ServerHandler for Calculators {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("jade-figuring", crate::VERSION))
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let tools = tools::definitions()
            .into_iter()
            .map(|(name, description, schema)| Tool::new(name, description, Arc::new(schema)))
            .collect();
        Ok(ListToolsResult::with_all_items(tools))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, McpError> {
        let name = request.name.to_string();
        let outcome = match tools::expression(request.arguments.as_ref()) {
            Err(e) => Err(e),
            Ok(expr) => {
                let task = tokio::task::spawn_blocking(move || tools::run(&name, &expr));
                match tokio::time::timeout(CALL_LIMIT, task).await {
                    Ok(Ok(r)) => r,
                    Ok(Err(_)) => Err("the calculation failed unexpectedly".into()),
                    Err(_) => Err("the calculation took too long and was abandoned".into()),
                }
            }
        };
        Ok(match outcome {
            Ok(text) => CallToolResult::success(vec![ContentBlock::text(text)]),
            Err(text) => CallToolResult::error(vec![ContentBlock::text(text)]),
        }
        .into())
    }
}

/// Settings read from the environment once, at start-up.
#[derive(Clone, Debug)]
pub struct Settings {
    pub allowed_hosts: Vec<String>,
    pub allowed_origins: Vec<String>,
}

impl Settings {
    pub fn from_env() -> Self {
        let list = |var: &str| -> Option<Vec<String>> {
            std::env::var(var).ok().map(|v| {
                v.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
        };
        Settings {
            allowed_hosts: list("JADE_ALLOWED_HOSTS")
                .unwrap_or_else(|| vec!["localhost".into(), "127.0.0.1".into(), "::1".into()]),
            allowed_origins: list("JADE_ALLOWED_ORIGINS").unwrap_or_default(),
        }
    }
}

pub fn app(settings: &Settings, started: Instant) -> Router {
    // `*` allows any Host, for a server reached by several names.
    let hosts = if settings.allowed_hosts.iter().any(|h| h == "*") {
        Vec::new()
    } else {
        settings.allowed_hosts.clone()
    };
    let mut config = StreamableHttpServerConfig::default()
        .with_legacy_session_mode(false)
        .with_json_response(true)
        .with_sse_keep_alive(None)
        .with_allowed_hosts(hosts)
        .with_max_request_body_bytes(64 * 1024);
    // A request from a web page carries an Origin; MCP clients send none.
    config = if settings.allowed_origins.is_empty() {
        config.enforce_origin_validation()
    } else {
        config.with_allowed_origins(settings.allowed_origins.clone())
    };
    let mcp: StreamableHttpService<Calculators, LocalSessionManager> =
        StreamableHttpService::new(|| Ok(Calculators), Default::default(), config);
    Router::new()
        .nest_service("/mcp", mcp)
        .route("/health", get(move || async move { Json(health(started)) }))
        .route("/admin/version", get(|| async { Json(version()) }))
}

pub fn health(started: Instant) -> Value {
    json!({"ok": true, "version": crate::VERSION, "uptime_seconds": started.elapsed().as_secs()})
}

fn version() -> Value {
    json!({
        "name": "jade-figuring",
        "version": crate::VERSION,
        "tools": tools::definitions().iter().map(|(n, ..)| *n).collect::<Vec<_>>(),
        "limits": {
            "expression_characters": calc::MAX_EXPRESSION_CHARS,
            "date_request_characters": dates::MAX_REQUEST_CHARS,
            "whole_numbers_shown_in_full_up_to_digits": calc::MAX_SHOWN_DIGITS,
            "whole_numbers_computed_up_to_digits": calc::MAX_INT_DIGITS,
            "significant_digits": calc::SIG_DIGITS,
            "calculation_seconds": calc::TIME_LIMIT.as_secs(),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn settings() -> Settings {
        Settings {
            allowed_hosts: vec!["localhost".into()],
            allowed_origins: vec![],
        }
    }

    async fn post(body: Value, origin: Option<&str>) -> (StatusCode, Value) {
        let mut req = Request::post("/mcp")
            .header("host", "localhost")
            .header("content-type", "application/json")
            .header("accept", "application/json, text/event-stream")
            .header("mcp-protocol-version", "2025-06-18");
        if let Some(o) = origin {
            req = req.header("origin", o);
        }
        let resp = app(&settings(), Instant::now())
            .oneshot(req.body(Body::from(body.to_string())).unwrap())
            .await
            .unwrap();
        let status = resp.status();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let text = String::from_utf8_lossy(&bytes).to_string();
        let json = serde_json::from_str(&text)
            .or_else(|_| {
                text.lines()
                    .filter_map(|l| l.strip_prefix("data:"))
                    .find_map(|d| serde_json::from_str(d.trim()).ok())
                    .ok_or(())
            })
            .unwrap_or(Value::Null);
        (status, json)
    }

    fn call(name: &str, args: Value) -> Value {
        json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": name, "arguments": args}})
    }

    #[tokio::test]
    async fn initialize_reports_the_server() {
        let (status, r) = post(
            json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
                "protocolVersion": "2025-06-18", "capabilities": {},
                "clientInfo": {"name": "test", "version": "0"}}}),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(r["result"]["serverInfo"]["name"], "jade-figuring");
        assert_eq!(r["result"]["serverInfo"]["version"], crate::VERSION);
    }

    #[tokio::test]
    async fn two_small_tools() {
        let (_, r) = post(
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}}),
            None,
        )
        .await;
        let tools = r["result"]["tools"].as_array().expect("a tool list");
        assert_eq!(tools.len(), 2);
        // The size as a client such as Peabody sends it: each tool as a chat
        // function, plus a `- name: description` line after the system prompt.
        let mut size = 0;
        for t in tools {
            let keys: Vec<&String> = t.as_object().unwrap().keys().collect();
            assert!(
                keys.iter()
                    .all(|k| ["name", "description", "inputSchema"].contains(&k.as_str())),
                "{keys:?}"
            );
            assert_eq!(
                t["inputSchema"],
                json!({"type": "object", "properties": {"expression": {"type": "string"}}, "required": ["expression"]})
            );
            let f = json!({"type": "function", "function": {
                "name": t["name"], "description": t["description"], "parameters": t["inputSchema"]}});
            size += f.to_string().chars().count();
            size += format!(
                "- {}: {}",
                t["name"].as_str().unwrap(),
                t["description"].as_str().unwrap()
            )
            .chars()
            .count();
        }
        assert!(
            size <= 800,
            "the tool definitions take {size} characters; the ceiling is 800"
        );
    }

    #[tokio::test]
    async fn both_tools_answer() {
        let (_, r) = post(
            call(
                "engineering_calculator",
                json!({"expression": "sin(30 degrees)"}),
            ),
            None,
        )
        .await;
        assert_eq!(r["result"]["content"][0]["text"], "sin(30°) = 0.5");
        assert_ne!(r["result"]["isError"], true);
        let (_, r) = post(
            call(
                "date_calculator",
                json!({"query": "days between 2026-10-31 and 2026-12-25"}),
            ),
            None,
        )
        .await;
        assert_eq!(
            r["result"]["content"][0]["text"],
            "from 2026-10-31 to 2026-12-25 = 55 days (7 weeks and 6 days)"
        );
        let (_, r) = post(call("engineering_calculator", json!({})), None).await;
        assert_eq!(r["result"]["isError"], true);
        assert!(
            r["result"]["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("no expression")
        );
        let (_, r) = post(
            call("engineering_calculator", json!({"expression": "9^9^9"})),
            None,
        )
        .await;
        assert_eq!(r["result"]["isError"], true);
    }

    #[tokio::test]
    async fn a_web_page_is_refused() {
        let (status, _) = post(
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}}),
            Some("http://evil.example"),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    #[test]
    fn health_has_the_handbook_shape() {
        let h = health(Instant::now());
        assert_eq!(h["ok"], true);
        assert_eq!(h["version"], crate::VERSION);
        assert!(h["uptime_seconds"].is_u64());
    }
}
