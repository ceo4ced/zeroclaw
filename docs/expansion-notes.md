# Expansion Notes: SpacetimeDB Documentation Gaps

Reviewed 2026-02-28 from a junior developer ("newbie") perspective.

---

## 1. Quickstart / Getting Started Guide

**Why it's needed:** The docs jump straight into architecture concepts without telling a new developer how to actually get the system running. There is no step-by-step "zero to working agent" guide. The Deployment Topology section in technical.html shows three commands but lacks prerequisite installation, expected output, and verification steps.

**What should be added:**
- Prerequisites checklist (Rust toolchain version, SpacetimeDB CLI install, OS support)
- Step-by-step quickstart: install SpacetimeDB, clone repo, publish module, configure agent, start worker, send first message
- Expected terminal output at each step so the reader can verify success
- A "You should see this" confirmation at the end

**Which file:** index.html (new section before the CTA row at the bottom)

---

## 2. Environment Variables Reference

**Why it's needed:** The worker uses environment variables like `SPACETIME_URI`, `SPACETIME_MODULE`, and `ZEROCLAW_AGENT_ID`, but these are only mentioned in passing inside a code block. A new developer wouldn't know which are required vs optional, what the defaults are, or whether there are additional env vars for things like log level or daemon mode.

**What should be added:**
- Complete table of all environment variables with name, required/optional, default value, and description
- Example `.env` file snippet

**Which file:** technical.html (new section after Deployment Topology)

---

## 3. End-to-End Request/Response Example

**Why it's needed:** The flow diagram in index.html is abstract. A newbie needs to see what an actual API call looks like -- the real JSON payloads going to the LLM, the real tool call structure coming back, and the real tool result being submitted. Without this, the "how" remains theoretical.

**What should be added:**
- Concrete example: user sends "What's the weather in Tokyo?", agent calls web_search tool, gets result, responds
- Show the actual JSON body sent to the LLM API
- Show the LLM response with tool_calls
- Show the tool result stored as a message
- Show the final response sent to the channel

**Which file:** technical.html (new section after Orchestration Loop)

---

## 4. Adding a Custom Tool Guide

**Why it's needed:** The docs explain the built-in tools (web_search, shell, etc.) but never explain how a developer would add their own custom tool. This is the most common extension task for anyone using the system. The tool_registry table exists but there is no walkthrough of registering a new HTTP tool or a new local tool.

**What should be added:**
- Step-by-step: adding a custom HTTP tool (register in tool_registry, add match arm in execute_http_tool)
- Step-by-step: adding a custom local tool (register via worker, add match arm in execute_local_tool)
- Example: adding a "get_stock_price" HTTP tool with full code

**Which file:** implementation.html (new section after HTTP Tool Execution)

---

## 5. Error Handling and Debugging Guide

**Why it's needed:** The docs show the happy path exclusively. A new developer will inevitably hit errors: LLM API key not set, worker not connected, tool execution failures, rate limits exceeded. There is no guidance on what error messages look like, where to find logs, or how to diagnose common problems.

**What should be added:**
- Common error scenarios and their symptoms (API key missing, worker disconnected, tool blocked by policy, iteration limit hit)
- Where logs appear (SpacetimeDB logs, worker stdout)
- How to inspect state via SpacetimeDB SQL queries (select from local_tool_requests where status = 'failed')
- Troubleshooting checklist

**Which file:** technical.html (new section after Memory Architecture)

---

## 6. Worker Reconnection and Resilience

**Why it's needed:** The docs mention the worker connects via WebSocket but never explain what happens when the connection drops. A new developer deploying this in production needs to know: Does the worker auto-reconnect? What happens to in-flight tool requests? Is there a heartbeat? What about stale worker entries in the workers table?

**What should be added:**
- Worker connection lifecycle: connect, register, subscribe, disconnect handling
- Reconnection strategy (exponential backoff, re-registration of tools)
- What happens to pending tool requests when a worker disconnects
- How on_disconnect cleans up the workers table
- How hygiene_schedule cleans up stale requests

**Which file:** technical.html (new section after Channel Architecture, or expand the existing Worker section)

---

## 7. Monitoring and Observability

**Why it's needed:** The overview page lists "Observability -- Every state change is a table mutation (subscribable)" as a design principle, but there is zero elaboration. A new developer has no idea how to actually monitor what the agent is doing, track conversation flow, or set up alerts.

**What should be added:**
- How to subscribe to table changes for monitoring
- Key tables to watch for operational health (local_tool_requests status, pending_sends status, workers table)
- Example subscription queries for common monitoring scenarios
- How process_results table can be used to track agent performance
- Integration points for external monitoring (webhook channel for alerts)

**Which file:** technical.html (new section near the end, after the expanded Worker section)

---

## 8. Security Policy Configuration Examples

**Why it's needed:** The security model section shows the SecurityPolicy struct and the is_tool_allowed logic, but never shows practical examples of configuring different security postures. A new developer wouldn't know how to set up a read-only agent, a fully autonomous agent, or an agent restricted to specific tools.

**What should be added:**
- Example: read-only agent (block shell and file_write)
- Example: full autonomy (empty allowlist, empty blocklist, auto mode)
- Example: restricted agent (only web_search and memory tools allowed)
- How to update security policy at runtime via reducer call
- Explanation of autonomy_level values and their behavioral impact

**Which file:** technical.html (expand the existing Security Model section)

---

## 9. How continue_conversation() Works

**Why it's needed:** The orchestration loop clearly shows that local tools cause the procedure to return with status "awaiting_local_tools", but the resumption path via continue_conversation() is only mentioned in passing. A new developer cannot trace the full lifecycle of a local tool request from creation to result submission to conversation resumption.

**What should be added:**
- The continue_conversation() procedure code (annotated)
- How submit_local_tool_result triggers continue_conversation
- The full lifecycle diagram: process_message -> awaiting -> worker executes -> submit_result -> continue_conversation -> LLM again
- What happens if multiple local tools are pending (batch vs sequential)

**Which file:** implementation.html (new section after the Worker section or expand orchestration_loop section)

---

## 10. Autonomy Levels Explained

**Why it's needed:** The security policy has an autonomy_level field with values "manual", "semi_auto", and "auto", but the docs never explain what these actually do behaviorally. Does "manual" require human approval for every tool call? Does "auto" skip confirmation? Where is the approval mechanism? A newbie has no idea.

**What should be added:**
- Definition of each autonomy level and its behavioral impact
- How manual mode works (if it queues for approval, where/how does the human approve?)
- How semi_auto differs from auto
- Whether autonomy level affects the orchestration loop or is informational only

**Which file:** technical.html (expand within the Security Model section)

---

## 11. SpacetimeDB Concepts Primer

**Why it's needed:** The docs assume familiarity with SpacetimeDB concepts like reducers, procedures, ProcedureContext, ctx.http, tables, subscriptions, Identity, and ScheduleAt. A developer who has never seen SpacetimeDB before will be lost by the second paragraph. The relationship between reducers (state changes) and procedures (I/O) is crucial but never explicitly defined.

**What should be added:**
- Brief glossary: what is a reducer, what is a procedure, what is a subscription, what is Identity
- Why procedures can do HTTP but reducers cannot
- How real-time subscriptions work (WebSocket push on table changes)
- Link to SpacetimeDB official docs for deeper reading

**Which file:** index.html (new section before the concept grid, serving as foundational context)

---

## 12. Channel Configuration Setup

**Why it's needed:** The channel architecture section explains the send mechanism but never shows how to actually configure a channel. The channel_configs table is mentioned but a new developer doesn't know how to add Telegram bot credentials, Discord webhook URLs, or Slack integration. This is a critical setup step that's completely missing.

**What should be added:**
- How to configure each channel type via the configure_channel reducer
- Required fields per channel type (Telegram: bot token + API base; Discord: webhook URL; Slack: webhook URL)
- Example reducer calls for setting up Telegram, Discord, and Slack
- How to test that a channel is working

**Which file:** technical.html (expand the existing Channel Architecture section)
