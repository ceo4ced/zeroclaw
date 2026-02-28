# Expansion Notes for SpacetimeDB Documentation

Reviewed 2026-02-28 from a junior developer ("newbie") perspective.

---

## 1. Quickstart / Getting Started Guide

**Why it's needed:** The docs jump straight into concepts and architecture. A newcomer has no idea how to actually get the system running. The deployment section in technical.html shows three commands but skips prerequisites, expected output, and verification steps.

**What should be added:**
- Prerequisites checklist: Rust toolchain version, SpacetimeDB CLI installation, supported OS
- Step-by-step "Hello World" flow: install SpacetimeDB CLI, publish the module, start the worker, send a test message
- Expected terminal output at each step so the reader knows they're on track
- A "verify it works" check at the end

**Which file:** `index.html` (new section before the CTA row at the bottom)

---

## 2. Environment Variables Reference

**Why it's needed:** The worker uses `SPACETIME_URI`, `SPACETIME_MODULE`, and `ZEROCLAW_AGENT_ID`, but these are only shown once in a deployment code snippet with no explanation. A newcomer wouldn't know which are required vs optional, what the defaults are, or what other env vars exist.

**What should be added:**
- Complete table of all environment variables
- Required vs optional designation
- Default values
- Description and example values for each

**Which file:** `technical.html` (new section after Deployment Topology)

---

## 3. End-to-End Request/Response Example

**Why it's needed:** The orchestration loop is described abstractly. A newcomer can't visualize what actual JSON goes over the wire. There's no concrete example showing "user sends X, LLM receives Y, agent responds Z."

**What should be added:**
- A concrete scenario: user asks "What's the weather in Tokyo?"
- Show the actual JSON that goes to the LLM API
- Show the LLM response with a tool_call for web_search
- Show the HTTP tool execution and its result
- Show the second LLM call with the tool result
- Show the final response back to the user

**Which file:** `technical.html` (new section after Orchestration Loop)

---

## 4. How to Add a Custom Tool

**Why it's needed:** The docs explain existing tools but give no guidance on adding new ones. The tool_registry table and execution_mode field hint at extensibility, but a newcomer wouldn't know the concrete steps.

**What should be added:**
- Step-by-step guide for adding an HTTP tool (e.g., a weather API tool)
- Step-by-step guide for adding a local tool (e.g., a database query tool)
- Where to register the tool (server init vs worker startup vs runtime reducer call)
- JSON schema format for tool parameters
- How the tool appears to the LLM

**Which file:** `implementation.html` (new section after HTTP Tool Execution)

---

## 5. Error Handling and Debugging

**Why it's needed:** The docs show happy paths exclusively. A newcomer will immediately hit errors (wrong API key, SpacetimeDB not running, worker can't connect) and has no idea how to diagnose them. Error states like "failed" in tool requests are mentioned but never explained.

**What should be added:**
- Common error scenarios and their symptoms
- How to read SpacetimeDB logs
- What happens when the LLM API key is missing or wrong
- What happens when the worker disconnects mid-tool-execution
- How to inspect the state of pending tool requests
- ProcessResult status codes and what each means

**Which file:** `technical.html` (new section after Security Model)

---

## 6. Worker Reconnection and Resilience

**Why it's needed:** The docs mention the worker subscribes to tables via WebSocket but say nothing about what happens on disconnect. A newcomer deploying this in production needs to know: Does the worker auto-reconnect? What happens to in-flight tool requests? Are there retries?

**What should be added:**
- Worker lifecycle: connect, register, subscribe, execute, disconnect
- What happens to pending tool requests when a worker disconnects
- Whether and how the worker reconnects automatically
- The on_disconnect handler and worker table cleanup
- How stale "processing" requests are recovered

**Which file:** `technical.html` (new section after Local Tools)

---

## 7. Observability and Monitoring

**Why it's needed:** One of the design principles listed is "Observability -- Every state change is a table mutation (subscribable)" but there's zero practical guidance on how to actually observe the system. A newcomer doesn't know how to watch what the agent is doing in real time.

**What should be added:**
- How to subscribe to tables for real-time monitoring
- Key tables to watch (local_tool_requests, pending_sends, messages, process_results)
- Using SpacetimeDB CLI commands to query state
- Example subscription queries for debugging
- The hygiene_schedule and what run_hygiene() cleans up

**Which file:** `technical.html` (new section after Deployment Topology / Environment Variables)

---

## 8. Security Policy Configuration Examples

**Why it's needed:** The security model section shows the struct and the authorization logic, but never shows a concrete example of setting up a restrictive policy. A newcomer reading about allowed_tools_json and blocked_tools_json needs to see actual reducer calls with real values.

**What should be added:**
- Example: create a read-only agent (block shell and file_write)
- Example: create a restricted agent (allow only web_search and memory tools)
- Example: set rate limits for a public-facing agent
- Show the actual reducer calls with parameters

**Which file:** `implementation.html` (new section after Security Policy Enforcement)

---

## 9. The continue_conversation() Flow

**Why it's needed:** The docs show that local tools cause the orchestration to pause and return "awaiting_local_tools", and they mention continue_conversation() resumes the loop. But there's no code or explanation showing how the worker triggers this, what the procedure does internally, or how the conversation state is restored.

**What should be added:**
- The continue_conversation() procedure signature and logic
- How the worker calls it after submitting tool results
- How the conversation state (tool_iteration counter) is managed
- What happens if only some tool results have been submitted

**Which file:** `implementation.html` (new section after the orchestration loop)

---

## 10. Channel Configuration Guide

**Why it's needed:** The channel architecture section shows that Telegram, Discord, Slack, and webhooks are supported, but never shows how to actually configure one. The channel_configs table is mentioned but its structure isn't shown, and there's no example of adding a Telegram bot.

**What should be added:**
- ChannelConfig table structure
- Step-by-step: configure a Telegram channel
- Step-by-step: configure a Discord webhook
- How to verify channel config is working
- What happens when channel config is missing (fallback to pending_sends)

**Which file:** `technical.html` (new section after Channel Architecture)

---

## 11. Glossary of Status Codes and State Machines

**Why it's needed:** The docs mention several status values throughout (pending, processing, completed, failed, awaiting_local_tools, iteration_limit, error) but never collect them in one place. A newcomer debugging a stuck conversation needs a quick reference for what each state means and what transitions are valid.

**What should be added:**
- ProcessResult status codes: completed, awaiting_local_tools, error, iteration_limit
- LocalToolRequest states: pending -> processing -> completed | failed
- PendingSend states: pending -> sent | failed
- Conversation.active field semantics
- State transition diagram for the full message lifecycle

**Which file:** `index.html` (new section after the Message Lifecycle flow diagram)

---

## 12. SpacetimeDB CLI Commands Cheat Sheet

**Why it's needed:** The docs reference `spacetime publish` and `spacetime generate` but a newcomer doesn't know what other SpacetimeDB CLI commands are useful for working with this runtime. Basic operations like querying tables, calling reducers manually, and checking module status are essential for development.

**What should be added:**
- Essential SpacetimeDB CLI commands for development
- How to call a reducer from the CLI
- How to query a table from the CLI
- How to check module status and connected clients
- How to view logs

**Which file:** `index.html` (new section before the CTA row, after the quickstart)
