//! ZeroClaw Worker — Thin I/O Bridge
//!
//! Connects to SpacetimeDB and handles only what procedures can't:
//!   - Local tool execution (shell, filesystem)
//!   - Channel listeners (polling for inbound messages)
//!   - Resuming conversations after local tool completion
//!
//! All LLM calls, HTTP tools, and HTTP channel sends are handled
//! inside SpacetimeDB procedures — the worker never touches those.
//!
//! Usage:
//!   SPACETIME_URI=ws://localhost:3000 \
//!   SPACETIME_MODULE=zeroclaw \
//!   ZEROCLAW_AGENT_ID=1 \
//!   cargo run
//!
//! Generate bindings first:
//!   cd ../server && spacetime generate --lang rust --out-dir ../worker/src/module_bindings --project-path .

// NOTE: In production, uncomment the module_bindings import below
// after running `spacetime generate`. The types and reducer/procedure
// call methods will be auto-generated from the server module.
//
// mod module_bindings;
// use module_bindings::*;

use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::env;
use std::io::{self, BufRead, Write};
use std::process::Command;

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let config = WorkerConfig::from_env()?;
    log::info!("ZeroClaw Worker starting");
    log::info!("  SpacetimeDB: {}/{}", config.spacetime_uri, config.module_name);
    log::info!("  Agent ID: {}", config.agent_id);
    log::info!("  Mode: {}", config.mode);

    match config.mode.as_str() {
        "cli" => run_cli_mode(&config),
        "daemon" => run_daemon_mode(&config),
        _ => bail!("Unknown mode: {}. Use 'cli' or 'daemon'.", config.mode),
    }
}

struct WorkerConfig {
    spacetime_uri: String,
    module_name: String,
    agent_id: u64,
    mode: String,
    worker_name: String,
}

impl WorkerConfig {
    fn from_env() -> Result<Self> {
        Ok(Self {
            spacetime_uri: env::var("SPACETIME_URI")
                .unwrap_or_else(|_| "ws://localhost:3000".into()),
            module_name: env::var("SPACETIME_MODULE")
                .unwrap_or_else(|_| "zeroclaw".into()),
            agent_id: env::var("ZEROCLAW_AGENT_ID")
                .unwrap_or_else(|_| "1".into())
                .parse()
                .context("ZEROCLAW_AGENT_ID must be a number")?,
            mode: env::var("ZEROCLAW_MODE")
                .unwrap_or_else(|_| "cli".into()),
            worker_name: env::var("ZEROCLAW_WORKER_NAME")
                .unwrap_or_else(|_| hostname()),
        })
    }
}

fn hostname() -> String {
    Command::new("hostname")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "zeroclaw-worker".into())
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  CLI Mode — Interactive local chat
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn run_cli_mode(config: &WorkerConfig) -> Result<()> {
    // In full implementation, this would:
    // 1. Connect to SpacetimeDB via spacetimedb-sdk
    // 2. Register as worker
    // 3. Register local tools (shell, file_read, file_write)
    // 4. Subscribe to local_tool_requests table
    // 5. Read user input, call process_message procedure
    // 6. Print responses
    // 7. If local tools needed, execute and call continue_conversation

    println!("ZeroClaw CLI (SpacetimeDB: {}/{})", config.spacetime_uri, config.module_name);
    println!("Agent ID: {}", config.agent_id);
    println!("Type your message, or /quit to exit.\n");

    // ── Pseudocode for the SpacetimeDB-connected flow ──
    //
    // let conn = DbConnection::builder()
    //     .with_uri(&config.spacetime_uri)
    //     .with_module_name(&config.module_name)
    //     .build()?;
    //
    // // Register as worker
    // conn.reducers.register_worker(config.worker_name.clone(), r#"["local_tools"]"#.into());
    //
    // // Register local tools
    // for (name, desc, schema) in local_tools() {
    //     conn.reducers.register_local_tool(name.into(), desc.into(), schema.into());
    // }
    //
    // // Subscribe to our tables
    // conn.subscription_builder()
    //     .subscribe(["SELECT * FROM local_tool_requests WHERE status = 'pending'"])
    //     .subscribe(["SELECT * FROM pending_sends WHERE status = 'pending'"])
    //     .on_applied(|_| log::info!("Subscriptions active"))
    //     .build();
    //
    // // Handle local tool requests reactively
    // conn.db.local_tool_requests().on_insert(|ctx, req| {
    //     if req.status == "pending" {
    //         let result = execute_local_tool(&req.tool_name, &req.parameters_json);
    //         ctx.reducers.submit_local_tool_result(req.id, result, true);
    //         // Resume conversation
    //         ctx.procedures.continue_conversation(req.conversation_id);
    //     }
    // });

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("you> ");
        stdout.flush()?;

        let mut input = String::new();
        if stdin.lock().read_line(&mut input)? == 0 {
            break; // EOF
        }
        let input = input.trim();

        if input.is_empty() {
            continue;
        }
        if input == "/quit" || input == "/exit" {
            println!("Goodbye.");
            break;
        }
        if input.starts_with('/') {
            handle_cli_command(input, config);
            continue;
        }

        // ── In full implementation: ──
        // let result = conn.procedures.process_message(
        //     config.agent_id,
        //     "cli".into(),
        //     config.worker_name.clone(),
        //     input.into(),
        // );
        //
        // match result.status.as_str() {
        //     "completed" => println!("\nzeroclaw> {}\n", result.response),
        //     "awaiting_local_tools" => {
        //         // Execute pending local tools
        //         let tools: Vec<ToolCallInfo> = serde_json::from_str(&result.pending_local_tools_json)?;
        //         for tool in &tools {
        //             let output = execute_local_tool(&tool.name, &tool.arguments);
        //             conn.reducers.submit_local_tool_result(tool.request_id, output, true);
        //         }
        //         // Resume
        //         let resumed = conn.procedures.continue_conversation(result.conversation_id);
        //         println!("\nzeroclaw> {}\n", resumed.response);
        //     }
        //     "error" => eprintln!("\n[error] {}\n", result.response),
        //     other => eprintln!("\n[{}] {}\n", other, result.response),
        // }

        // Placeholder until SpacetimeDB connection is wired up:
        println!("\n[Worker not connected to SpacetimeDB yet]");
        println!("[Would call process_message({}, cli, {}, {:?})]", config.agent_id, config.worker_name, input);
        println!();
    }

    Ok(())
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  Daemon Mode — Background worker for local tools + channel polling
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn run_daemon_mode(config: &WorkerConfig) -> Result<()> {
    // In full implementation:
    // 1. Connect to SpacetimeDB
    // 2. Register as worker
    // 3. Register local tools
    // 4. Subscribe to local_tool_requests and pending_sends
    // 5. React to table changes:
    //    - local_tool_requests: execute tool, submit result, call continue_conversation
    //    - pending_sends: send via non-HTTP channels (e.g., iMessage, IRC)
    // 6. Optionally poll channel APIs for inbound messages
    //    and call process_message procedure for each

    log::info!("Daemon mode: would connect to SpacetimeDB and process local tool requests");
    log::info!("Not yet implemented — run in 'cli' mode for now");

    // Block forever (daemon)
    // loop { std::thread::park(); }

    Ok(())
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  Local Tool Execution
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn execute_local_tool(tool_name: &str, params_json: &str) -> String {
    match tool_name {
        "shell" => execute_shell(params_json),
        "file_read" => execute_file_read(params_json),
        "file_write" => execute_file_write(params_json),
        _ => format!("Unknown local tool: {}", tool_name),
    }
}

fn execute_shell(params_json: &str) -> String {
    #[derive(serde::Deserialize)]
    struct Params {
        command: String,
        #[serde(default = "default_timeout")]
        timeout_secs: u64,
    }
    fn default_timeout() -> u64 {
        30
    }

    let params: Params = match serde_json::from_str(params_json) {
        Ok(p) => p,
        Err(e) => return format!("Invalid params: {}", e),
    };

    log::info!("Executing shell: {}", params.command);

    let output = Command::new("sh")
        .arg("-c")
        .arg(&params.command)
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            let mut result = String::new();

            if !stdout.is_empty() {
                result.push_str(&stdout);
            }
            if !stderr.is_empty() {
                if !result.is_empty() {
                    result.push('\n');
                }
                result.push_str("stderr: ");
                result.push_str(&stderr);
            }
            if result.is_empty() {
                result = format!("Exit code: {}", out.status.code().unwrap_or(-1));
            }

            // Truncate large output
            if result.len() > 8000 {
                result.truncate(8000);
                result.push_str("\n... (truncated)");
            }

            result
        }
        Err(e) => format!("Shell execution failed: {}", e),
    }
}

fn execute_file_read(params_json: &str) -> String {
    #[derive(serde::Deserialize)]
    struct Params {
        path: String,
        #[serde(default)]
        offset: usize,
        #[serde(default = "default_limit")]
        limit: usize,
    }
    fn default_limit() -> usize {
        2000
    }

    let params: Params = match serde_json::from_str(params_json) {
        Ok(p) => p,
        Err(e) => return format!("Invalid params: {}", e),
    };

    match std::fs::read_to_string(&params.path) {
        Ok(content) => {
            let lines: Vec<&str> = content.lines().collect();
            let start = params.offset.min(lines.len());
            let end = (start + params.limit).min(lines.len());

            lines[start..end]
                .iter()
                .enumerate()
                .map(|(i, line)| format!("{:>4} {}", start + i + 1, line))
                .collect::<Vec<_>>()
                .join("\n")
        }
        Err(e) => format!("Read error: {}", e),
    }
}

fn execute_file_write(params_json: &str) -> String {
    #[derive(serde::Deserialize)]
    struct Params {
        path: String,
        content: String,
    }

    let params: Params = match serde_json::from_str(params_json) {
        Ok(p) => p,
        Err(e) => return format!("Invalid params: {}", e),
    };

    match std::fs::write(&params.path, &params.content) {
        Ok(()) => format!("Written {} bytes to {}", params.content.len(), params.path),
        Err(e) => format!("Write error: {}", e),
    }
}

/// Tool definitions for local tools registered with SpacetimeDB.
fn local_tools() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        (
            "shell",
            "Execute a shell command on the local system",
            r#"{"type":"object","properties":{"command":{"type":"string","description":"Shell command to execute"},"timeout_secs":{"type":"integer","description":"Timeout in seconds","default":30}},"required":["command"]}"#,
        ),
        (
            "file_read",
            "Read a file from the local filesystem",
            r#"{"type":"object","properties":{"path":{"type":"string","description":"Absolute path to file"},"offset":{"type":"integer","description":"Line offset (0-based)","default":0},"limit":{"type":"integer","description":"Max lines to read","default":2000}},"required":["path"]}"#,
        ),
        (
            "file_write",
            "Write content to a file on the local filesystem",
            r#"{"type":"object","properties":{"path":{"type":"string","description":"Absolute path to file"},"content":{"type":"string","description":"Content to write"}},"required":["path","content"]}"#,
        ),
    ]
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  CLI Commands
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn handle_cli_command(input: &str, config: &WorkerConfig) {
    match input {
        "/help" => {
            println!("Commands:");
            println!("  /help       — Show this help");
            println!("  /status     — Show connection status");
            println!("  /memory     — List stored memories");
            println!("  /tools      — List available tools");
            println!("  /quit       — Exit");
        }
        "/status" => {
            println!("SpacetimeDB: {}/{}", config.spacetime_uri, config.module_name);
            println!("Agent ID: {}", config.agent_id);
            println!("Worker: {}", config.worker_name);
            // In full implementation: show connection state, subscriptions, etc.
        }
        "/memory" | "/tools" => {
            println!("[Would query SpacetimeDB tables]");
        }
        _ => {
            println!("Unknown command: {}", input);
        }
    }
}
