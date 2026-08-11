//! A real Model Context Protocol (MCP) server over stdio — a protocol
//! adapter over the existing `AgentContext` capability
//! (navigate/query/click/input/get_text/submit_form/go_back/go_forward/
//! get_current_url/get_history), the same capability the `/agent` HTTP
//! endpoint already exposes (`src/server.rs::dispatch_agent_action`). This
//! gives it a second, MCP-native transport so a real MCP client (Claude
//! Desktop, Claude Code, etc.) can drive Himalayas directly as a set of
//! tools, rather than needing a bespoke HTTP integration for each one.
//!
//! Protocol framing/message shapes were verified against the live spec
//! (modelcontextprotocol.io/specification/2025-06-18) rather than guessed:
//! stdio transport is newline-delimited JSON-RPC 2.0, one message per
//! line, no embedded newlines within a message. `stdout` **MUST** carry
//! only valid MCP messages — this already holds without extra work here,
//! since `init_logging` (`main.rs`) already sends all `tracing` output to
//! `stderr`, and this module never uses `println!`.
//!
//! One session for the whole process lifetime: stdio MCP is inherently
//! single-client-per-process (the client launches this as a subprocess),
//! unlike the HTTP endpoint's per-`session_id` map in `server.rs`, so
//! there's no equivalent multi-tenant concern to handle here.

use crate::api::AgentContext;
use crate::browser::Browser;
use anyhow::Result;
use serde_json::{Value, json};
use std::io::Write;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};

const PROTOCOL_VERSION: &str = "2025-06-18";

pub async fn run_stdio_server() -> Result<()> {
    let browser = Arc::new(Browser::new()?);
    let session = browser.create_session("mcp".to_string())?;
    let ctx = Arc::new(AgentContext::new(session, browser));

    let stdin = tokio::io::stdin();
    let mut lines = BufReader::new(stdin).lines();

    while let Some(line) = lines.next_line().await? {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let request: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => {
                write_message(&json!({
                    "jsonrpc": "2.0",
                    "id": Value::Null,
                    "error": { "code": -32700, "message": format!("parse error: {e}") }
                }))?;
                continue;
            }
        };

        if let Some(response) = handle_message(&ctx, request).await {
            write_message(&response)?;
        }
    }

    Ok(())
}

fn write_message(value: &Value) -> Result<()> {
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    writeln!(lock, "{}", serde_json::to_string(value)?)?;
    lock.flush()?;
    Ok(())
}

/// `None` for anything that shouldn't get a response — JSON-RPC
/// *notifications* (no `id` field, e.g. `notifications/initialized`) never
/// get one; a request always does, even on error.
async fn handle_message(ctx: &Arc<AgentContext>, request: Value) -> Option<Value> {
    let id = request.get("id").cloned();
    let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("");

    match method {
        "initialize" => {
            let id = id?;
            Some(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": PROTOCOL_VERSION,
                    "capabilities": { "tools": { "listChanged": false } },
                    "serverInfo": { "name": "himalayas", "version": env!("CARGO_PKG_VERSION") },
                    "instructions": "Himalayas Browser agent tools: navigate, query, click, input, get_text, submit_form, go_back, go_forward, get_current_url, get_history. One shared browsing session for this connection's lifetime — navigate() first, then query/click/input/get_text/submit_form act on whatever page navigate() last loaded.",
                }
            }))
        }
        "notifications/initialized" => None,
        "ping" => {
            let id = id?;
            Some(json!({ "jsonrpc": "2.0", "id": id, "result": {} }))
        }
        "tools/list" => {
            let id = id?;
            Some(json!({ "jsonrpc": "2.0", "id": id, "result": { "tools": tool_definitions() } }))
        }
        "tools/call" => {
            let id = id?;
            let params = request.get("params").cloned().unwrap_or(Value::Null);
            Some(handle_tool_call(ctx, id, params).await)
        }
        _ => {
            // Unknown *notifications* (no id) are silently ignored, per
            // spec convention for forward compatibility; unknown
            // *requests* get a real JSON-RPC error.
            let id = id?;
            Some(json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32601, "message": format!("method not found: {method}") }
            }))
        }
    }
}

fn tool_definitions() -> Value {
    let string_param = |desc: &str| json!({ "type": "string", "description": desc });
    let schema = |props: Value, required: &[&str]| {
        json!({ "type": "object", "properties": props, "required": required })
    };

    json!([
        {
            "name": "navigate",
            "description": "Load a URL in the shared browsing session and return the page's semantic DOM (elements, links, forms).",
            "inputSchema": schema(json!({ "url": string_param("The URL to navigate to") }), &["url"]),
        },
        {
            "name": "query",
            "description": "Run a CSS selector against the currently loaded page and return matching elements.",
            "inputSchema": schema(json!({ "selector": string_param("A CSS selector, e.g. '.price' or 'div.card > a'") }), &["selector"]),
        },
        {
            "name": "click",
            "description": "Click an element (by the id returned from navigate/query) — follows a link or submits a button.",
            "inputSchema": schema(json!({ "element_id": string_param("An element id from a prior navigate/query result") }), &["element_id"]),
        },
        {
            "name": "input",
            "description": "Type a value into a form field (by element id).",
            "inputSchema": schema(json!({
                "element_id": string_param("An element id from a prior navigate/query result"),
                "value": string_param("The text to enter"),
            }), &["element_id", "value"]),
        },
        {
            "name": "get_text",
            "description": "Read the text content of an element (by element id).",
            "inputSchema": schema(json!({ "element_id": string_param("An element id from a prior navigate/query result") }), &["element_id"]),
        },
        {
            "name": "submit_form",
            "description": "Submit a form (by element id) using its current field values.",
            "inputSchema": schema(json!({ "form_id": string_param("A form element id from a prior navigate/query result") }), &["form_id"]),
        },
        {
            "name": "go_back",
            "description": "Navigate back to the previous page in this session's history.",
            "inputSchema": schema(json!({}), &[]),
        },
        {
            "name": "go_forward",
            "description": "Navigate forward to a later page in this session's history.",
            "inputSchema": schema(json!({ "url": string_param("The URL to go forward to") }), &["url"]),
        },
        {
            "name": "get_current_url",
            "description": "Return the URL of the page currently loaded in this session.",
            "inputSchema": schema(json!({}), &[]),
        },
        {
            "name": "get_history",
            "description": "Return the list of URLs visited in this session so far.",
            "inputSchema": schema(json!({}), &[]),
        },
    ])
}

fn tool_call_error(id: &Value, message: String) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": -32602, "message": message }
    })
}

async fn handle_tool_call(ctx: &Arc<AgentContext>, id: Value, params: Value) -> Value {
    let name = params.get("name").and_then(|n| n.as_str()).unwrap_or_default();
    let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));

    let arg_str = |key: &str| -> Result<String, Value> {
        args.get(key)
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .ok_or_else(|| tool_call_error(&id, format!("missing '{key}' argument")))
    };

    let result: Result<Value> = match name {
        "navigate" => {
            let url = match arg_str("url") {
                Ok(v) => v,
                Err(e) => return e,
            };
            ctx.navigate(&url).await.and_then(|dom| Ok(serde_json::to_value(dom)?))
        }
        "query" => {
            let selector = match arg_str("selector") {
                Ok(v) => v,
                Err(e) => return e,
            };
            ctx.query(&selector).await.and_then(|els| Ok(serde_json::to_value(els)?))
        }
        "click" => {
            let element_id = match arg_str("element_id") {
                Ok(v) => v,
                Err(e) => return e,
            };
            ctx.click(&element_id).await.and_then(|dom| Ok(serde_json::to_value(dom)?))
        }
        "input" => {
            let element_id = match arg_str("element_id") {
                Ok(v) => v,
                Err(e) => return e,
            };
            let value = match arg_str("value") {
                Ok(v) => v,
                Err(e) => return e,
            };
            ctx.input(&element_id, &value).await.map(|_| json!({ "ok": true }))
        }
        "get_text" => {
            let element_id = match arg_str("element_id") {
                Ok(v) => v,
                Err(e) => return e,
            };
            ctx.get_text(&element_id).await.map(|text| json!(text))
        }
        "submit_form" => {
            let form_id = match arg_str("form_id") {
                Ok(v) => v,
                Err(e) => return e,
            };
            ctx.submit_form(&form_id).await.and_then(|dom| Ok(serde_json::to_value(dom)?))
        }
        "go_back" => ctx.go_back().map(|_| json!({ "ok": true })),
        "go_forward" => {
            let url = match arg_str("url") {
                Ok(v) => v,
                Err(e) => return e,
            };
            ctx.go_forward(url).map(|_| json!({ "ok": true }))
        }
        "get_current_url" => Ok(json!(ctx.get_current_url())),
        "get_history" => Ok(json!(ctx.get_history())),
        _ => {
            return json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32602, "message": format!("unknown tool: {name}") }
            });
        }
    };

    // Tool *execution* errors (a bad selector, a missing element, a fetch
    // failure) are reported via `isError: true` in a normal result, not a
    // JSON-RPC protocol error — per spec, protocol errors are for things
    // like "unknown tool"/"invalid params" (handled above via early
    // `return`), not for a tool that ran but failed at what it was asked.
    match result {
        Ok(value) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": { "content": [{ "type": "text", "text": value.to_string() }], "isError": false }
        }),
        Err(e) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": { "content": [{ "type": "text", "text": e.to_string() }], "isError": true }
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_ctx() -> Arc<AgentContext> {
        let browser = Arc::new(Browser::new().unwrap());
        let session = browser.create_session("test".to_string()).unwrap();
        Arc::new(AgentContext::new(session, browser))
    }

    #[tokio::test]
    async fn initialize_returns_the_negotiated_protocol_version_and_server_info() {
        let ctx = test_ctx();
        let request = json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": { "protocolVersion": PROTOCOL_VERSION, "capabilities": {}, "clientInfo": { "name": "test", "version": "0" } }
        });
        let response = handle_message(&ctx, request).await.unwrap();
        assert_eq!(response["result"]["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(response["result"]["serverInfo"]["name"], "himalayas");
        assert_eq!(response["id"], 1);
    }

    #[tokio::test]
    async fn initialized_notification_gets_no_response() {
        let ctx = test_ctx();
        let request = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
        assert!(handle_message(&ctx, request).await.is_none());
    }

    #[tokio::test]
    async fn tools_list_includes_all_ten_agent_context_methods() {
        let ctx = test_ctx();
        let request = json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" });
        let response = handle_message(&ctx, request).await.unwrap();
        let tools = response["result"]["tools"].as_array().unwrap();
        let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        for expected in [
            "navigate",
            "query",
            "click",
            "input",
            "get_text",
            "submit_form",
            "go_back",
            "go_forward",
            "get_current_url",
            "get_history",
        ] {
            assert!(names.contains(&expected), "missing tool: {expected}");
        }
    }

    #[tokio::test]
    async fn tools_call_with_missing_argument_is_a_protocol_error_not_a_tool_error() {
        let ctx = test_ctx();
        let request = json!({
            "jsonrpc": "2.0", "id": 3, "method": "tools/call",
            "params": { "name": "navigate", "arguments": {} }
        });
        let response = handle_message(&ctx, request).await.unwrap();
        assert_eq!(response["error"]["code"], -32602);
        assert!(response.get("result").is_none());
    }

    #[tokio::test]
    async fn tools_call_get_current_url_reflects_the_shared_session() {
        let ctx = test_ctx();
        let request = json!({
            "jsonrpc": "2.0", "id": 4, "method": "tools/call",
            "params": { "name": "get_current_url", "arguments": {} }
        });
        let response = handle_message(&ctx, request).await.unwrap();
        assert_eq!(response["result"]["isError"], false);
        // A fresh session has no page loaded yet — just confirms the round
        // trip works end-to-end without needing a live network fetch.
        assert_eq!(response["result"]["content"][0]["type"], "text");
    }

    #[tokio::test]
    async fn tools_call_with_unknown_tool_name_is_a_protocol_error() {
        let ctx = test_ctx();
        let request = json!({
            "jsonrpc": "2.0", "id": 5, "method": "tools/call",
            "params": { "name": "not_a_real_tool", "arguments": {} }
        });
        let response = handle_message(&ctx, request).await.unwrap();
        assert_eq!(response["error"]["code"], -32602);
    }

    #[tokio::test]
    async fn unknown_request_method_is_method_not_found() {
        let ctx = test_ctx();
        let request = json!({ "jsonrpc": "2.0", "id": 6, "method": "not/a/real/method" });
        let response = handle_message(&ctx, request).await.unwrap();
        assert_eq!(response["error"]["code"], -32601);
    }
}
