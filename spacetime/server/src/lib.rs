//! ZeroClaw Brain — SpacetimeDB Module
//!
//! The entire agent runtime lives here. Reducers handle state.
//! Procedures handle outbound I/O (LLM calls, channel sends, HTTP tools).
//! The only external component is a thin worker for local-only operations
//! (shell exec, filesystem, hardware).
//!
//! Deploy: `spacetime publish zeroclaw`
//! Worker bindings: `spacetime generate --lang rust --out-dir ../worker/src/module_bindings --project-path .`

use spacetimedb::{Identity, ProcedureContext, ReducerContext, ScheduleAt, Table, Timestamp};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  Tables
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// An agent identity: model config, system prompt, limits.
/// Multiple agents can coexist — each customer/tenant gets their own.
#[spacetimedb::table(name = agents, public)]
pub struct Agent {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub name: String,
    /// Provider-prefixed model ID, e.g. "anthropic/claude-sonnet-4-20250514"
    pub model: String,
    /// LLM API base URL for this agent's provider
    pub api_base_url: String,
    /// API key (stored encrypted in production — plaintext here for POC)
    pub api_key: String,
    pub system_prompt: String,
    pub max_tool_iterations: u32,
    pub max_history_messages: u32,
    pub owner: Identity,
    pub created_at: Timestamp,
}

/// A conversation session. Scoped to agent + channel + sender.
#[spacetimedb::table(name = conversations, public)]
pub struct Conversation {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub agent_id: u64,
    pub channel: String,
    pub sender: String,
    pub active: bool,
    pub tool_iteration: u32,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// A single message. Roles: system, user, assistant, tool.
#[spacetimedb::table(name = messages, public)]
pub struct Message {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub conversation_id: u64,
    pub role: String,
    pub content: String,
    /// Set when role="tool" — the tool_call ID this responds to.
    pub tool_call_id: String,
    /// JSON tool_calls array when the assistant wants to use tools.
    pub tool_calls_json: String,
    pub created_at: Timestamp,
}

/// Persistent memory. Survives across conversations. Shared per agent.
#[spacetimedb::table(name = memories, public)]
pub struct Memory {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub agent_id: u64,
    #[unique]
    pub key: String,
    pub content: String,
    /// core | daily | conversation
    pub category: String,
    pub session_id: String,
    pub created_at: Timestamp,
}

/// Tool requests that need LOCAL execution (shell, filesystem, hardware).
/// HTTP-based tools are executed directly inside procedures via ctx.http.
#[spacetimedb::table(name = local_tool_requests, public)]
pub struct LocalToolRequest {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub conversation_id: u64,
    pub tool_call_id: String,
    pub tool_name: String,
    pub parameters_json: String,
    /// pending → processing → completed | failed
    pub status: String,
    pub result: String,
    pub claimed_by: String,
    pub created_at: Timestamp,
}

/// Outbound messages waiting to be sent (for non-HTTP channels only).
/// HTTP channels (Telegram, Discord, Slack) are sent directly via procedures.
#[spacetimedb::table(name = pending_sends, public)]
pub struct PendingSend {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub conversation_id: u64,
    pub channel: String,
    pub target: String,
    pub content: String,
    /// pending → sent | failed
    pub status: String,
    pub created_at: Timestamp,
}

/// Per-agent security policy. Deny-by-default.
#[spacetimedb::table(name = security_policies, public)]
pub struct SecurityPolicy {
    #[primary_key]
    pub agent_id: u64,
    /// manual | semi_auto | auto
    pub autonomy_level: String,
    /// JSON array of allowed tool names. Empty = all allowed.
    pub allowed_tools_json: String,
    /// JSON array of blocked tool names.
    pub blocked_tools_json: String,
    pub max_tool_iterations: u32,
    pub rate_limit_per_minute: u32,
}

/// Connected workers (for local tool execution).
#[spacetimedb::table(name = workers, public)]
pub struct Worker {
    #[primary_key]
    pub identity: Identity,
    pub name: String,
    pub capabilities_json: String,
    pub last_heartbeat: Timestamp,
    pub connected_at: Timestamp,
}

/// Tool registry: available tools and their JSON schemas.
#[spacetimedb::table(name = tool_registry, public)]
pub struct ToolRegistryEntry {
    #[primary_key]
    pub name: String,
    pub description: String,
    pub parameters_schema_json: String,
    /// "http" = executed inside procedures, "local" = needs worker
    pub execution_mode: String,
}

/// Scheduled hygiene for memory cleanup.
#[spacetimedb::table(name = hygiene_schedule, scheduled(run_hygiene))]
pub struct HygieneSchedule {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: ScheduleAt,
}

/// Channel configuration for outbound HTTP sends.
#[spacetimedb::table(name = channel_configs, public)]
pub struct ChannelConfig {
    #[primary_key]
    pub name: String,
    /// telegram | discord | slack | webhook
    pub channel_type: String,
    /// Bot token or webhook URL
    pub credential: String,
    /// API base URL (e.g., "https://api.telegram.org")
    pub api_base_url: String,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  JSON types for serde
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct ToolCall {
    id: String,
    name: String,
    arguments: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct LlmMessage {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<LlmToolCall>>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct LlmToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: LlmFunction,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct LlmFunction {
    name: String,
    arguments: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct LlmApiRequest {
    model: String,
    messages: Vec<LlmMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<serde_json::Value>>,
}

#[derive(serde::Deserialize)]
struct LlmApiResponse {
    choices: Vec<LlmChoice>,
}

#[derive(serde::Deserialize)]
struct LlmChoice {
    message: LlmResponseMessage,
}

#[derive(serde::Deserialize)]
struct LlmResponseMessage {
    content: Option<String>,
    tool_calls: Option<Vec<LlmToolCall>>,
}

/// Return value from process_message / continue_conversation procedures.
#[derive(spacetimedb::SpacetimeType, serde::Serialize)]
pub struct ProcessResult {
    pub status: String,
    pub response: String,
    pub conversation_id: u64,
    /// Non-empty if local tools are pending and need worker execution.
    pub pending_local_tools_json: String,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  Lifecycle Reducers
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[spacetimedb::reducer(init)]
pub fn init(ctx: &ReducerContext) {
    // Default agent
    ctx.db.agents().insert(Agent {
        id: 0,
        name: "zeroclaw".into(),
        model: "claude-sonnet-4-20250514".into(),
        api_base_url: "https://api.anthropic.com/v1/messages".into(),
        api_key: String::new(), // Set via update_agent
        system_prompt: "You are ZeroClaw, an autonomous agent. \
            Be helpful, precise, and security-conscious."
            .into(),
        max_tool_iterations: 10,
        max_history_messages: 50,
        owner: ctx.sender,
        created_at: ctx.timestamp,
    });

    ctx.db.security_policies().insert(SecurityPolicy {
        agent_id: 1,
        autonomy_level: "semi_auto".into(),
        allowed_tools_json: "[]".into(),
        blocked_tools_json: "[]".into(),
        max_tool_iterations: 10,
        rate_limit_per_minute: 60,
    });

    // Register built-in HTTP tools
    for (name, desc, schema) in builtin_http_tools() {
        ctx.db.tool_registry().insert(ToolRegistryEntry {
            name: name.into(),
            description: desc.into(),
            parameters_schema_json: schema.into(),
            execution_mode: "http".into(),
        });
    }

    // Hourly hygiene
    use std::time::Duration;
    ctx.db.hygiene_schedule().insert(HygieneSchedule {
        scheduled_id: 0,
        scheduled_at: Duration::from_secs(3600).into(),
    });

    log::info!("ZeroClaw brain initialized");
}

#[spacetimedb::reducer(client_connected)]
pub fn on_connect(ctx: &ReducerContext) {
    log::info!("Client connected: {:?}", ctx.sender);
}

#[spacetimedb::reducer(client_disconnected)]
pub fn on_disconnect(ctx: &ReducerContext) {
    if let Some(w) = ctx.db.workers().identity().find(&ctx.sender) {
        ctx.db.workers().identity().delete(&ctx.sender);
        log::info!("Worker disconnected: {}", w.name);
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  State Management Reducers
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[spacetimedb::reducer]
pub fn create_agent(
    ctx: &ReducerContext,
    name: String,
    model: String,
    api_base_url: String,
    api_key: String,
    system_prompt: String,
) -> Result<(), String> {
    if name.is_empty() {
        return Err("Agent name required".into());
    }
    ctx.db.agents().insert(Agent {
        id: 0,
        name,
        model,
        api_base_url,
        api_key,
        system_prompt,
        max_tool_iterations: 10,
        max_history_messages: 50,
        owner: ctx.sender,
        created_at: ctx.timestamp,
    });
    Ok(())
}

#[spacetimedb::reducer]
pub fn update_agent(
    ctx: &ReducerContext,
    agent_id: u64,
    model: String,
    api_base_url: String,
    api_key: String,
    system_prompt: String,
    max_tool_iterations: u32,
    max_history_messages: u32,
) -> Result<(), String> {
    let agent = ctx.db.agents().id().find(&agent_id).ok_or("Agent not found")?;
    if agent.owner != ctx.sender {
        return Err("Not the agent owner".into());
    }
    ctx.db.agents().id().delete(&agent_id);
    ctx.db.agents().insert(Agent {
        model,
        api_base_url,
        api_key,
        system_prompt,
        max_tool_iterations,
        max_history_messages,
        ..agent
    });
    Ok(())
}

#[spacetimedb::reducer]
pub fn configure_channel(
    ctx: &ReducerContext,
    name: String,
    channel_type: String,
    credential: String,
    api_base_url: String,
) {
    if ctx.db.channel_configs().name().find(&name).is_some() {
        ctx.db.channel_configs().name().delete(&name);
    }
    ctx.db.channel_configs().insert(ChannelConfig {
        name,
        channel_type,
        credential,
        api_base_url,
    });
}

#[spacetimedb::reducer]
pub fn register_worker(ctx: &ReducerContext, name: String, capabilities_json: String) {
    if ctx.db.workers().identity().find(&ctx.sender).is_some() {
        ctx.db.workers().identity().delete(&ctx.sender);
    }
    ctx.db.workers().insert(Worker {
        identity: ctx.sender,
        name: name.clone(),
        capabilities_json,
        last_heartbeat: ctx.timestamp,
        connected_at: ctx.timestamp,
    });
    log::info!("Worker registered: {}", name);
}

#[spacetimedb::reducer]
pub fn register_local_tool(
    ctx: &ReducerContext,
    name: String,
    description: String,
    parameters_schema_json: String,
) {
    if ctx.db.tool_registry().name().find(&name).is_some() {
        ctx.db.tool_registry().name().delete(&name);
    }
    ctx.db.tool_registry().insert(ToolRegistryEntry {
        name,
        description,
        parameters_schema_json,
        execution_mode: "local".into(),
    });
}

#[spacetimedb::reducer]
pub fn store_memory(
    ctx: &ReducerContext,
    agent_id: u64,
    key: String,
    content: String,
    category: String,
) -> Result<(), String> {
    if key.is_empty() || content.is_empty() {
        return Err("Key and content required".into());
    }
    if let Some(existing) = ctx.db.memories().key().find(&key) {
        ctx.db.memories().key().delete(&key);
        ctx.db.memories().insert(Memory {
            content,
            category,
            created_at: ctx.timestamp,
            ..existing
        });
    } else {
        ctx.db.memories().insert(Memory {
            id: 0,
            agent_id,
            key,
            content,
            category,
            session_id: String::new(),
            created_at: ctx.timestamp,
        });
    }
    Ok(())
}

#[spacetimedb::reducer]
pub fn forget_memory(ctx: &ReducerContext, key: String) -> Result<(), String> {
    if ctx.db.memories().key().find(&key).is_some() {
        ctx.db.memories().key().delete(&key);
        Ok(())
    } else {
        Err("Memory not found".into())
    }
}

#[spacetimedb::reducer]
pub fn set_security_policy(
    ctx: &ReducerContext,
    agent_id: u64,
    autonomy_level: String,
    allowed_tools_json: String,
    blocked_tools_json: String,
    max_tool_iterations: u32,
    rate_limit_per_minute: u32,
) -> Result<(), String> {
    let agent = ctx.db.agents().id().find(&agent_id).ok_or("Agent not found")?;
    if agent.owner != ctx.sender {
        return Err("Not the agent owner".into());
    }
    if ctx.db.security_policies().agent_id().find(&agent_id).is_some() {
        ctx.db.security_policies().agent_id().delete(&agent_id);
    }
    ctx.db.security_policies().insert(SecurityPolicy {
        agent_id,
        autonomy_level,
        allowed_tools_json,
        blocked_tools_json,
        max_tool_iterations,
        rate_limit_per_minute,
    });
    Ok(())
}

/// Worker reports a local tool execution result.
#[spacetimedb::reducer]
pub fn submit_local_tool_result(
    ctx: &ReducerContext,
    request_id: u64,
    result: String,
    success: bool,
) -> Result<(), String> {
    let req = ctx
        .db
        .local_tool_requests()
        .id()
        .find(&request_id)
        .ok_or("Request not found")?;

    ctx.db.local_tool_requests().id().delete(&request_id);
    ctx.db.local_tool_requests().insert(LocalToolRequest {
        status: if success { "completed" } else { "failed" }.into(),
        result,
        ..req
    });
    Ok(())
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  Procedures — Outbound I/O lives here
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// The main orchestration loop. Handles an inbound user message end-to-end:
///   1. Store message (tx)
///   2. Call LLM API (HTTP via ctx.http)
///   3. If HTTP tools needed → execute via ctx.http, loop back to 2
///   4. If local tools needed → write LocalToolRequest rows, return pending
///   5. If done → send response to channel (HTTP), return complete
///
/// For conversations with no local tools, the entire interaction completes
/// in a single procedure call with zero worker involvement.
#[spacetimedb::procedure]
pub fn process_message(
    ctx: &mut ProcedureContext,
    agent_id: u64,
    channel: String,
    sender: String,
    content: String,
) -> ProcessResult {
    // ── 1. Store inbound message ──
    let (conv_id, agent_model, agent_api_base, agent_api_key, agent_prompt, max_iter, max_hist) =
        ctx.with_tx(|tx| {
            let agent = tx.db.agents().id().find(&agent_id).expect("Agent not found");
            let conv_id = find_or_create_conv(tx, agent_id, &channel, &sender);

            tx.db.messages().insert(Message {
                id: 0,
                conversation_id: conv_id,
                role: "user".into(),
                content: content.clone(),
                tool_call_id: String::new(),
                tool_calls_json: String::new(),
                created_at: tx.timestamp,
            });

            // Auto-save to memory
            if content.len() > 20 {
                let key = format!("conv_{}_{}", conv_id, tx.timestamp);
                ctx_store_memory(tx, agent_id, &key, &content, "conversation");
            }

            (
                conv_id,
                agent.model.clone(),
                agent.api_base_url.clone(),
                agent.api_key.clone(),
                agent.system_prompt.clone(),
                agent.max_tool_iterations,
                agent.max_history_messages,
            )
        });

    // ── 2. Orchestration loop ──
    orchestration_loop(
        ctx,
        conv_id,
        agent_id,
        &agent_model,
        &agent_api_base,
        &agent_api_key,
        &agent_prompt,
        max_iter,
        max_hist,
        &channel,
        &sender,
    )
}

/// Resume orchestration after local tool results are submitted.
/// The worker calls this after executing local tools and writing results
/// via submit_local_tool_result.
#[spacetimedb::procedure]
pub fn continue_conversation(
    ctx: &mut ProcedureContext,
    conversation_id: u64,
) -> ProcessResult {
    // Verify all local tools are done
    let any_pending = ctx.with_tx(|tx| {
        tx.db
            .local_tool_requests()
            .iter()
            .any(|r| r.conversation_id == conversation_id && r.status == "pending")
    });
    if any_pending {
        return ProcessResult {
            status: "error".into(),
            response: "Local tool requests still pending".into(),
            conversation_id,
            pending_local_tools_json: String::new(),
        };
    }

    // Store tool results as messages
    ctx.with_tx(|tx| {
        let completed: Vec<LocalToolRequest> = tx
            .db
            .local_tool_requests()
            .iter()
            .filter(|r| {
                r.conversation_id == conversation_id
                    && (r.status == "completed" || r.status == "failed")
            })
            .collect();

        for req in &completed {
            tx.db.messages().insert(Message {
                id: 0,
                conversation_id,
                role: "tool".into(),
                content: req.result.clone(),
                tool_call_id: req.tool_call_id.clone(),
                tool_calls_json: String::new(),
                created_at: tx.timestamp,
            });
            tx.db.local_tool_requests().id().delete(&req.id);
        }
    });

    // Get agent info for this conversation
    let (agent_id, agent_model, agent_api_base, agent_api_key, agent_prompt, max_iter, max_hist, channel, sender) =
        ctx.with_tx(|tx| {
            let conv = tx
                .db
                .conversations()
                .id()
                .find(&conversation_id)
                .expect("Conversation not found");
            let agent = tx
                .db
                .agents()
                .id()
                .find(&conv.agent_id)
                .expect("Agent not found");
            (
                conv.agent_id,
                agent.model.clone(),
                agent.api_base_url.clone(),
                agent.api_key.clone(),
                agent.system_prompt.clone(),
                agent.max_tool_iterations,
                agent.max_history_messages,
                conv.channel.clone(),
                conv.sender.clone(),
            )
        });

    orchestration_loop(
        ctx,
        conversation_id,
        agent_id,
        &agent_model,
        &agent_api_base,
        &agent_api_key,
        &agent_prompt,
        max_iter,
        max_hist,
        &channel,
        &sender,
    )
}

/// Send a message to a configured HTTP channel (Telegram, Discord, Slack, webhook).
#[spacetimedb::procedure]
pub fn send_channel_message(
    ctx: &mut ProcedureContext,
    channel_name: String,
    target: String,
    content: String,
) -> String {
    let config = ctx.with_tx(|tx| tx.db.channel_configs().name().find(&channel_name));

    let Some(config) = config else {
        return format!("Channel '{}' not configured", channel_name);
    };

    let result = match config.channel_type.as_str() {
        "telegram" => send_telegram(ctx, &config, &target, &content),
        "discord" => send_discord_webhook(ctx, &config, &content),
        "slack" => send_slack_webhook(ctx, &config, &content),
        "webhook" => send_generic_webhook(ctx, &config, &target, &content),
        other => Err(format!("Unknown channel type: {}", other)),
    };

    match result {
        Ok(()) => "sent".into(),
        Err(e) => format!("send failed: {}", e),
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  Scheduled Reducers
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[spacetimedb::reducer]
pub fn run_hygiene(ctx: &ReducerContext, _schedule: HygieneSchedule) {
    let mut cleaned = 0u64;

    let done: Vec<u64> = ctx
        .db
        .local_tool_requests()
        .iter()
        .filter(|r| r.status == "completed" || r.status == "failed")
        .map(|r| r.id)
        .collect();
    for id in &done {
        ctx.db.local_tool_requests().id().delete(id);
        cleaned += 1;
    }

    let sent: Vec<u64> = ctx
        .db
        .pending_sends()
        .iter()
        .filter(|m| m.status == "sent")
        .map(|m| m.id)
        .collect();
    for id in &sent {
        ctx.db.pending_sends().id().delete(id);
        cleaned += 1;
    }

    if cleaned > 0 {
        log::info!("Hygiene: cleaned {} stale rows", cleaned);
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  Internal: Orchestration Loop
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[allow(clippy::too_many_arguments)]
fn orchestration_loop(
    ctx: &mut ProcedureContext,
    conv_id: u64,
    agent_id: u64,
    model: &str,
    api_base: &str,
    api_key: &str,
    system_prompt: &str,
    max_iter: u32,
    max_hist: u32,
    channel: &str,
    sender: &str,
) -> ProcessResult {
    for iteration in 0..max_iter {
        // Build message history inside a transaction
        let (messages, tools) = ctx.with_tx(|tx| {
            let msgs = build_messages(tx, conv_id, system_prompt, agent_id, max_hist);
            let tools = build_tools(tx);
            (msgs, tools)
        });

        // ── Call LLM via HTTP ──
        let llm_result = call_llm(ctx, api_base, api_key, model, &messages, &tools);

        let (response_content, tool_calls) = match llm_result {
            Ok(r) => r,
            Err(e) => {
                let err_msg = format!("LLM API error: {}", e);
                log::error!("{}", err_msg);
                return ProcessResult {
                    status: "error".into(),
                    response: err_msg,
                    conversation_id: conv_id,
                    pending_local_tools_json: String::new(),
                };
            }
        };

        // Store assistant message
        let tc_json = if tool_calls.is_empty() {
            String::new()
        } else {
            serde_json::to_string(&tool_calls).unwrap_or_default()
        };

        ctx.with_tx(|tx| {
            tx.db.messages().insert(Message {
                id: 0,
                conversation_id: conv_id,
                role: "assistant".into(),
                content: response_content.clone(),
                tool_call_id: String::new(),
                tool_calls_json: tc_json,
                created_at: tx.timestamp,
            });

            // Update iteration counter
            if let Some(conv) = tx.db.conversations().id().find(&conv_id) {
                tx.db.conversations().id().delete(&conv_id);
                tx.db.conversations().insert(Conversation {
                    tool_iteration: iteration + 1,
                    updated_at: tx.timestamp,
                    ..conv
                });
            }
        });

        if tool_calls.is_empty() {
            // ── Done: send final response ──
            try_send_response(ctx, conv_id, channel, sender, &response_content);

            return ProcessResult {
                status: "completed".into(),
                response: response_content,
                conversation_id: conv_id,
                pending_local_tools_json: String::new(),
            };
        }

        // ── Execute tool calls ──
        let policy = ctx.with_tx(|tx| tx.db.security_policies().agent_id().find(&agent_id));
        let mut local_pending: Vec<ToolCall> = Vec::new();

        for call in &tool_calls {
            if !is_tool_allowed(&policy, &call.name) {
                ctx.with_tx(|tx| {
                    tx.db.messages().insert(Message {
                        id: 0,
                        conversation_id: conv_id,
                        role: "tool".into(),
                        content: format!(
                            "Error: tool '{}' blocked by security policy",
                            call.name
                        ),
                        tool_call_id: call.id.clone(),
                        tool_calls_json: String::new(),
                        created_at: tx.timestamp,
                    });
                });
                continue;
            }

            let exec_mode = ctx.with_tx(|tx| {
                tx.db
                    .tool_registry()
                    .name()
                    .find(&call.name)
                    .map(|t| t.execution_mode.clone())
                    .unwrap_or_else(|| "local".into())
            });

            if exec_mode == "http" {
                // Execute HTTP tool inline via procedure
                let result = execute_http_tool(ctx, &call.name, &call.arguments);
                ctx.with_tx(|tx| {
                    tx.db.messages().insert(Message {
                        id: 0,
                        conversation_id: conv_id,
                        role: "tool".into(),
                        content: result,
                        tool_call_id: call.id.clone(),
                        tool_calls_json: String::new(),
                        created_at: tx.timestamp,
                    });
                });
            } else {
                // Needs worker — write request row
                ctx.with_tx(|tx| {
                    tx.db.local_tool_requests().insert(LocalToolRequest {
                        id: 0,
                        conversation_id: conv_id,
                        tool_call_id: call.id.clone(),
                        tool_name: call.name.clone(),
                        parameters_json: call.arguments.clone(),
                        status: "pending".into(),
                        result: String::new(),
                        claimed_by: String::new(),
                        created_at: tx.timestamp,
                    });
                });
                local_pending.push(call.clone());
            }
        }

        // If local tools are pending, return control to the worker
        if !local_pending.is_empty() {
            let pending_json = serde_json::to_string(&local_pending).unwrap_or_default();
            return ProcessResult {
                status: "awaiting_local_tools".into(),
                response: response_content,
                conversation_id: conv_id,
                pending_local_tools_json: pending_json,
            };
        }

        // All tools were HTTP — loop continues to next LLM call
    }

    // Hit iteration limit
    let msg = "[Tool iteration limit reached]".to_string();
    try_send_response(ctx, conv_id, channel, sender, &msg);
    ProcessResult {
        status: "iteration_limit".into(),
        response: msg,
        conversation_id: conv_id,
        pending_local_tools_json: String::new(),
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  Internal: LLM API Call
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn call_llm(
    ctx: &mut ProcedureContext,
    api_base: &str,
    api_key: &str,
    model: &str,
    messages: &[LlmMessage],
    tools: &[serde_json::Value],
) -> Result<(String, Vec<ToolCall>), String> {
    let request = LlmApiRequest {
        model: model.into(),
        messages: messages.to_vec(),
        tools: if tools.is_empty() {
            None
        } else {
            Some(tools.to_vec())
        },
    };

    let body = serde_json::to_string(&request).map_err(|e| format!("Serialize error: {}", e))?;

    // Anthropic Messages API format
    let response = ctx
        .http
        .request("POST", api_base)
        .header("content-type", "application/json")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .body(body.as_bytes())
        .send()
        .map_err(|e| format!("HTTP error: {:?}", e))?;

    let status = response.status();
    let response_body = String::from_utf8_lossy(response.body()).to_string();

    if status != 200 {
        return Err(format!("LLM API returned {}: {}", status, response_body));
    }

    // Parse response (OpenAI-compatible format)
    let parsed: LlmApiResponse =
        serde_json::from_str(&response_body).map_err(|e| format!("Parse error: {}", e))?;

    let choice = parsed.choices.first().ok_or("No choices in response")?;
    let content = choice.message.content.clone().unwrap_or_default();

    let tool_calls: Vec<ToolCall> = choice
        .message
        .tool_calls
        .as_ref()
        .map(|tcs| {
            tcs.iter()
                .map(|tc| ToolCall {
                    id: tc.id.clone(),
                    name: tc.function.name.clone(),
                    arguments: tc.function.arguments.clone(),
                })
                .collect()
        })
        .unwrap_or_default();

    Ok((content, tool_calls))
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  Internal: HTTP Tool Execution
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn execute_http_tool(ctx: &mut ProcedureContext, tool_name: &str, args_json: &str) -> String {
    match tool_name {
        "web_search" => execute_web_search(ctx, args_json),
        "http_request" => execute_http_request(ctx, args_json),
        "memory_store" => execute_memory_store(ctx, args_json),
        "memory_recall" => execute_memory_recall(ctx, args_json),
        _ => format!("Unknown HTTP tool: {}", tool_name),
    }
}

fn execute_web_search(ctx: &mut ProcedureContext, args_json: &str) -> String {
    #[derive(serde::Deserialize)]
    struct Args {
        query: String,
    }

    let args: Args = match serde_json::from_str(args_json) {
        Ok(a) => a,
        Err(e) => return format!("Invalid arguments: {}", e),
    };

    // Use a search API (DuckDuckGo HTML as fallback)
    let url = format!(
        "https://html.duckduckgo.com/html/?q={}",
        urlencoded(&args.query)
    );

    match ctx.http.request("GET", &url).send() {
        Ok(resp) => {
            let body = String::from_utf8_lossy(resp.body()).to_string();
            // Return first 2000 chars of results
            if body.len() > 2000 {
                body[..2000].to_string()
            } else {
                body
            }
        }
        Err(e) => format!("Search failed: {:?}", e),
    }
}

fn execute_http_request(ctx: &mut ProcedureContext, args_json: &str) -> String {
    #[derive(serde::Deserialize)]
    struct Args {
        url: String,
        method: Option<String>,
        body: Option<String>,
    }

    let args: Args = match serde_json::from_str(args_json) {
        Ok(a) => a,
        Err(e) => return format!("Invalid arguments: {}", e),
    };

    let method = args.method.as_deref().unwrap_or("GET");

    let mut req = ctx.http.request(method, &args.url);
    if let Some(body) = &args.body {
        req = req.header("content-type", "application/json").body(body.as_bytes());
    }

    match req.send() {
        Ok(resp) => {
            let status = resp.status();
            let body = String::from_utf8_lossy(resp.body()).to_string();
            if body.len() > 4000 {
                format!("HTTP {} (truncated):\n{}", status, &body[..4000])
            } else {
                format!("HTTP {}:\n{}", status, body)
            }
        }
        Err(e) => format!("Request failed: {:?}", e),
    }
}

fn execute_memory_store(ctx: &mut ProcedureContext, args_json: &str) -> String {
    #[derive(serde::Deserialize)]
    struct Args {
        key: String,
        content: String,
        #[serde(default = "default_category")]
        category: String,
        #[serde(default)]
        agent_id: u64,
    }
    fn default_category() -> String {
        "core".into()
    }

    let args: Args = match serde_json::from_str(args_json) {
        Ok(a) => a,
        Err(e) => return format!("Invalid arguments: {}", e),
    };

    let agent_id = if args.agent_id == 0 { 1 } else { args.agent_id };

    ctx.with_tx(|tx| {
        ctx_store_memory(tx, agent_id, &args.key, &args.content, &args.category);
    });

    format!("Stored memory: {}", args.key)
}

fn execute_memory_recall(ctx: &mut ProcedureContext, args_json: &str) -> String {
    #[derive(serde::Deserialize)]
    struct Args {
        query: String,
        #[serde(default)]
        agent_id: u64,
    }

    let args: Args = match serde_json::from_str(args_json) {
        Ok(a) => a,
        Err(e) => return format!("Invalid arguments: {}", e),
    };

    let agent_id = if args.agent_id == 0 { 1 } else { args.agent_id };

    ctx.with_tx(|tx| {
        let words: Vec<&str> = args
            .query
            .split_whitespace()
            .filter(|w| w.len() > 3)
            .collect();

        let mut results: Vec<String> = Vec::new();
        for mem in tx.db.memories().iter() {
            if mem.agent_id != agent_id {
                continue;
            }
            let lower = mem.content.to_lowercase();
            if words.iter().any(|w| lower.contains(&w.to_lowercase())) {
                results.push(format!("[{}] ({}): {}", mem.key, mem.category, mem.content));
                if results.len() >= 10 {
                    break;
                }
            }
        }

        if results.is_empty() {
            "No memories found matching query".into()
        } else {
            results.join("\n")
        }
    })
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  Internal: Channel Sends
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn try_send_response(
    ctx: &mut ProcedureContext,
    conv_id: u64,
    channel: &str,
    target: &str,
    content: &str,
) {
    let config = ctx.with_tx(|tx| tx.db.channel_configs().name().find(&channel.to_string()));

    if let Some(config) = config {
        let result = match config.channel_type.as_str() {
            "telegram" => send_telegram(ctx, &config, target, content),
            "discord" => send_discord_webhook(ctx, &config, content),
            "slack" => send_slack_webhook(ctx, &config, content),
            "webhook" => send_generic_webhook(ctx, &config, target, content),
            _ => Ok(()), // Unknown type — fall through to PendingSend
        };
        if result.is_ok() {
            return;
        }
        log::error!("Channel send failed, queuing as PendingSend");
    }

    // Fallback: queue for worker to send (non-HTTP channel or send failure)
    ctx.with_tx(|tx| {
        tx.db.pending_sends().insert(PendingSend {
            id: 0,
            conversation_id: conv_id,
            channel: channel.into(),
            target: target.into(),
            content: content.into(),
            status: "pending".into(),
            created_at: tx.timestamp,
        });
    });
}

fn send_telegram(
    ctx: &mut ProcedureContext,
    config: &ChannelConfig,
    chat_id: &str,
    text: &str,
) -> Result<(), String> {
    let url = format!(
        "{}/bot{}/sendMessage",
        config.api_base_url, config.credential
    );
    let body = serde_json::json!({
        "chat_id": chat_id,
        "text": text,
        "parse_mode": "Markdown",
    });
    let body_str = serde_json::to_string(&body).map_err(|e| e.to_string())?;

    ctx.http
        .request("POST", &url)
        .header("content-type", "application/json")
        .body(body_str.as_bytes())
        .send()
        .map_err(|e| format!("{:?}", e))?;
    Ok(())
}

fn send_discord_webhook(
    ctx: &mut ProcedureContext,
    config: &ChannelConfig,
    content: &str,
) -> Result<(), String> {
    let body = serde_json::json!({ "content": content });
    let body_str = serde_json::to_string(&body).map_err(|e| e.to_string())?;

    ctx.http
        .request("POST", &config.credential) // credential = webhook URL
        .header("content-type", "application/json")
        .body(body_str.as_bytes())
        .send()
        .map_err(|e| format!("{:?}", e))?;
    Ok(())
}

fn send_slack_webhook(
    ctx: &mut ProcedureContext,
    config: &ChannelConfig,
    text: &str,
) -> Result<(), String> {
    let body = serde_json::json!({ "text": text });
    let body_str = serde_json::to_string(&body).map_err(|e| e.to_string())?;

    ctx.http
        .request("POST", &config.credential) // credential = webhook URL
        .header("content-type", "application/json")
        .body(body_str.as_bytes())
        .send()
        .map_err(|e| format!("{:?}", e))?;
    Ok(())
}

fn send_generic_webhook(
    ctx: &mut ProcedureContext,
    config: &ChannelConfig,
    target: &str,
    content: &str,
) -> Result<(), String> {
    let body = serde_json::json!({
        "target": target,
        "content": content,
    });
    let body_str = serde_json::to_string(&body).map_err(|e| e.to_string())?;

    ctx.http
        .request("POST", &config.credential)
        .header("content-type", "application/json")
        .body(body_str.as_bytes())
        .send()
        .map_err(|e| format!("{:?}", e))?;
    Ok(())
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  Internal: Helpers
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// TxContext is whatever type `with_tx` gives us — using ReducerContext
/// as the bound since both share the same .db accessor.
fn find_or_create_conv(
    tx: &spacetimedb::TxContext,
    agent_id: u64,
    channel: &str,
    sender: &str,
) -> u64 {
    for conv in tx.db.conversations().iter() {
        if conv.agent_id == agent_id
            && conv.channel == channel
            && conv.sender == sender
            && conv.active
        {
            return conv.id;
        }
    }
    let conv = tx.db.conversations().insert(Conversation {
        id: 0,
        agent_id,
        channel: channel.into(),
        sender: sender.into(),
        active: true,
        tool_iteration: 0,
        created_at: tx.timestamp,
        updated_at: tx.timestamp,
    });
    conv.id
}

fn ctx_store_memory(
    tx: &spacetimedb::TxContext,
    agent_id: u64,
    key: &str,
    content: &str,
    category: &str,
) {
    let key_owned = key.to_string();
    if let Some(existing) = tx.db.memories().key().find(&key_owned) {
        tx.db.memories().key().delete(&key_owned);
        tx.db.memories().insert(Memory {
            content: content.into(),
            category: category.into(),
            created_at: tx.timestamp,
            ..existing
        });
    } else {
        tx.db.memories().insert(Memory {
            id: 0,
            agent_id,
            key: key_owned,
            content: content.into(),
            category: category.into(),
            session_id: String::new(),
            created_at: tx.timestamp,
        });
    }
}

fn build_messages(
    tx: &spacetimedb::TxContext,
    conv_id: u64,
    system_prompt: &str,
    agent_id: u64,
    max_hist: u32,
) -> Vec<LlmMessage> {
    let mut out: Vec<LlmMessage> = Vec::new();

    // System prompt
    out.push(LlmMessage {
        role: "system".into(),
        content: system_prompt.into(),
        tool_call_id: None,
        tool_calls: None,
    });

    // Inject relevant memories
    let memory_ctx = recall_context(tx, agent_id, conv_id);
    if !memory_ctx.is_empty() {
        out.push(LlmMessage {
            role: "system".into(),
            content: format!("Relevant memories:\n{}", memory_ctx),
            tool_call_id: None,
            tool_calls: None,
        });
    }

    // Conversation messages
    let mut msgs: Vec<Message> = tx
        .db
        .messages()
        .iter()
        .filter(|m| m.conversation_id == conv_id)
        .collect();
    msgs.sort_by_key(|m| m.id);

    let max = max_hist as usize;
    let start = msgs.len().saturating_sub(max);

    for msg in &msgs[start..] {
        let tool_calls = if !msg.tool_calls_json.is_empty() && msg.tool_calls_json != "[]" {
            serde_json::from_str::<Vec<LlmToolCall>>(&msg.tool_calls_json).ok()
        } else {
            None
        };

        let tool_call_id = if msg.tool_call_id.is_empty() {
            None
        } else {
            Some(msg.tool_call_id.clone())
        };

        out.push(LlmMessage {
            role: msg.role.clone(),
            content: msg.content.clone(),
            tool_call_id,
            tool_calls,
        });
    }

    out
}

fn recall_context(tx: &spacetimedb::TxContext, agent_id: u64, conv_id: u64) -> String {
    let recent: String = tx
        .db
        .messages()
        .iter()
        .filter(|m| m.conversation_id == conv_id && m.role == "user")
        .map(|m| m.content.clone())
        .collect::<Vec<_>>()
        .join(" ");

    if recent.is_empty() {
        return String::new();
    }

    let words: Vec<&str> = recent.split_whitespace().filter(|w| w.len() > 3).collect();
    let mut hits: Vec<String> = Vec::new();

    for mem in tx.db.memories().iter() {
        if mem.agent_id != agent_id || mem.category == "conversation" {
            continue;
        }
        let lower = mem.content.to_lowercase();
        if words.iter().any(|w| lower.contains(&w.to_lowercase())) {
            hits.push(format!("- [{}] {}", mem.key, mem.content));
            if hits.len() >= 5 {
                break;
            }
        }
    }

    hits.join("\n")
}

fn build_tools(tx: &spacetimedb::TxContext) -> Vec<serde_json::Value> {
    tx.db
        .tool_registry()
        .iter()
        .map(|t| {
            let params: serde_json::Value =
                serde_json::from_str(&t.parameters_schema_json).unwrap_or(serde_json::json!({}));
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": &t.name,
                    "description": &t.description,
                    "parameters": params,
                }
            })
        })
        .collect()
}

fn is_tool_allowed(policy: &Option<SecurityPolicy>, tool_name: &str) -> bool {
    let Some(p) = policy else { return true };

    if let Ok(blocked) = serde_json::from_str::<Vec<String>>(&p.blocked_tools_json) {
        if blocked.iter().any(|b| b == tool_name) {
            return false;
        }
    }
    if let Ok(allowed) = serde_json::from_str::<Vec<String>>(&p.allowed_tools_json) {
        if !allowed.is_empty() && !allowed.iter().any(|a| a == tool_name) {
            return false;
        }
    }
    true
}

fn urlencoded(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => out.push(c),
            ' ' => out.push('+'),
            _ => {
                let mut buf = [0u8; 4];
                let encoded = c.encode_utf8(&mut buf);
                for b in encoded.bytes() {
                    out.push('%');
                    out.push_str(&format!("{:02X}", b));
                }
            }
        }
    }
    out
}

fn builtin_http_tools() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        (
            "web_search",
            "Search the web for information",
            r#"{"type":"object","properties":{"query":{"type":"string","description":"Search query"}},"required":["query"]}"#,
        ),
        (
            "http_request",
            "Make an HTTP request to a URL",
            r#"{"type":"object","properties":{"url":{"type":"string","description":"URL to request"},"method":{"type":"string","description":"HTTP method (GET, POST, etc.)","default":"GET"},"body":{"type":"string","description":"Request body (for POST/PUT)"}},"required":["url"]}"#,
        ),
        (
            "memory_store",
            "Store information in persistent memory",
            r#"{"type":"object","properties":{"key":{"type":"string","description":"Memory key"},"content":{"type":"string","description":"Content to remember"},"category":{"type":"string","description":"Category: core, daily, or conversation","default":"core"}},"required":["key","content"]}"#,
        ),
        (
            "memory_recall",
            "Search persistent memory for relevant information",
            r#"{"type":"object","properties":{"query":{"type":"string","description":"Search query"}},"required":["query"]}"#,
        ),
    ]
}
