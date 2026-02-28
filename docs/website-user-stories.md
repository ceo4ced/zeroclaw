# SpaceClaw Website — User Stories

> These user stories define the full website experience and will drive page generation,
> component design, and feature prioritization for the SpaceClaw platform website.
>
> **Auth model:** SSO via Google and GitHub (no password-based accounts).
> **Design direction:** Simple, minimalist, multi-page platform.

---

## Epic 1: Public Website (Unauthenticated Visitor)

### 1.1 Landing Page

- **US-1.1.1** As a visitor, I want to see a clear hero section explaining what SpaceClaw does in one sentence, so I can immediately understand the product value.
- **US-1.1.2** As a visitor, I want to see a concise feature grid (agent orchestration, 25+ LLM providers, 18+ channels, persistent memory, security policies, hardware peripherals), so I can assess capability breadth at a glance.
- **US-1.1.3** As a visitor, I want to see a "How It Works" section with 3–4 steps (sign up, configure agent, connect channels, deploy), so I understand the path from signup to running agent.
- **US-1.1.4** As a visitor, I want to see social proof (testimonials, beta user count, GitHub stars), so I can gauge community traction.
- **US-1.1.5** As a visitor, I want a prominent "Get Started Free" call-to-action that takes me to the SSO login flow, so I can begin onboarding immediately.
- **US-1.1.6** As a visitor, I want a persistent top navigation bar with links to Features, Pricing, Docs, and Login, so I can navigate the site from any page.
- **US-1.1.7** As a visitor, I want a footer with links to Terms of Service, Privacy Policy, Status Page, GitHub repo, and contact/support, so I can find legal and support resources.

### 1.2 Features Page

- **US-1.2.1** As a visitor, I want a dedicated features page that explains each core capability in detail (agent orchestration, provider support, channel integrations, memory, security, tools, hardware peripherals), so I can evaluate whether SpaceClaw fits my use case.
- **US-1.2.2** As a visitor, I want to see the full list of supported LLM providers (OpenAI, Anthropic, Gemini, Mistral, Groq, Ollama, etc.) with logos, so I know my preferred provider is supported.
- **US-1.2.3** As a visitor, I want to see the full list of supported channels (Telegram, Discord, Slack, WhatsApp, Matrix, Email, etc.) with logos, so I know my preferred messaging platform is supported.
- **US-1.2.4** As a visitor, I want to see a brief explanation of the SpacetimeDB architecture advantage (zero-ops, database-as-runtime, real-time multi-agent sync), so I understand the technical differentiator.
- **US-1.2.5** As a visitor, I want to see the open-core model explained (MIT/Apache 2.0 core, proprietary SaaS layer), so I understand what's free to self-host vs. what the platform provides.

### 1.3 Pricing Page

- **US-1.3.1** As a visitor, I want to see all four pricing tiers (Free, Builder $29/mo, Team $99/mo, Enterprise $499–999/mo) displayed side-by-side in a comparison table, so I can quickly identify the right tier.
- **US-1.3.2** As a visitor, I want to see per-tier limits clearly stated (agent count, memory, agent-hours, included seats, support level), so there are no surprises after signup.
- **US-1.3.3** As a visitor, I want to see overage pricing (per agent-hour, per GB storage, per tool execution) clearly disclosed below the tier table, so I can estimate costs at scale.
- **US-1.3.4** As a visitor, I want to see the per-seat add-on pricing (Builder $12/seat, Team $15/seat, Enterprise $25/seat), so I can plan team costs.
- **US-1.3.5** As a visitor, I want to see the on-demand compute rate card (Standard $0.05/hr, High-memory $0.10/hr, Multi-agent $0.15/hr, Dedicated $0.25/hr), so I can budget for compute-intensive workloads.
- **US-1.3.6** As a visitor, I want to see a "Start Free" button on the Free tier and "Start 14-Day Trial" buttons on paid tiers, so I can begin without a credit card.
- **US-1.3.7** As a visitor, I want to see a "Contact Sales" button on the Enterprise tier, so I can initiate a custom deal conversation.
- **US-1.3.8** As a visitor, I want to see an FAQ section addressing common pricing questions (Can I switch tiers? What happens if I exceed limits? Is there an annual discount? Can I self-host instead?), so my concerns are preemptively addressed.

### 1.4 Documentation Link

- **US-1.4.1** As a visitor, I want a "Docs" link in the top navigation that takes me to the documentation hub, so I can evaluate the platform's depth before signing up.

---

## Epic 2: Authentication (SSO)

### 2.1 Login / Signup

- **US-2.1.1** As a visitor, I want to sign up using my Google account via OAuth 2.0, so I don't need to create a separate password.
- **US-2.1.2** As a visitor, I want to sign up using my GitHub account via OAuth 2.0, so I can use my developer identity.
- **US-2.1.3** As a visitor, I want the login page to show two clear buttons ("Continue with Google" and "Continue with GitHub") and nothing else, so the auth flow is frictionless.
- **US-2.1.4** As a new user, I want my account to be automatically created on first SSO login (no separate registration step), so I can reach the dashboard immediately.
- **US-2.1.5** As a new user, I want to be placed on the Free tier by default after first login, so I can start using the platform without payment.
- **US-2.1.6** As a returning user, I want to be redirected to my dashboard after SSO login, so I don't have to navigate there manually.

### 2.2 Session Management

- **US-2.2.1** As a logged-in user, I want my session to persist across browser tabs and survive page refreshes, so I don't have to re-authenticate constantly.
- **US-2.2.2** As a logged-in user, I want my session to expire after a reasonable inactivity period (e.g. 7 days) and be prompted to re-authenticate, so my account stays secure.
- **US-2.2.3** As a logged-in user, I want a "Log Out" option accessible from any page, so I can end my session explicitly.

---

## Epic 3: Dashboard (Authenticated Home)

### 3.1 Dashboard Overview

- **US-3.1.1** As a logged-in user, I want to see a dashboard overview showing my current tier, agent count (used/limit), agent-hours consumed this billing cycle, and memory usage, so I have an at-a-glance status of my account.
- **US-3.1.2** As a logged-in user, I want to see a list of my agents with their current status (running, stopped, errored), so I know which agents are active.
- **US-3.1.3** As a logged-in user, I want to see recent activity (last 10 agent actions, channel messages processed, tool executions), so I can monitor what my agents have been doing.
- **US-3.1.4** As a logged-in user, I want quick-action buttons ("Create Agent", "Connect Channel", "View Logs"), so I can jump to common tasks without navigating menus.
- **US-3.1.5** As a Free-tier user, I want to see a non-intrusive upgrade prompt showing what I'd unlock on Builder tier, so I'm aware of the upgrade path without feeling pressured.

---

## Epic 4: Agent Management

### 4.1 Create Agent

- **US-4.1.1** As a logged-in user, I want to create a new agent by providing a name, selecting a provider (from the 25+ supported), and entering my API key or choosing managed proxy, so I can get an agent running quickly.
- **US-4.1.2** As a logged-in user, I want to write or paste a system prompt for my agent during creation, so the agent has clear instructions from the start.
- **US-4.1.3** As a logged-in user, I want to select an autonomy level (Supervised, Constrained, Autonomous) during agent creation, so I control how independently the agent operates.
- **US-4.1.4** As a logged-in user, I want to see my remaining agent slots (e.g. "2 of 5 agents used") before creating, so I know if I need to upgrade or remove an existing agent.

### 4.2 Agent Detail / Config

- **US-4.2.1** As a logged-in user, I want to view a detail page for each agent showing its configuration (provider, model, system prompt, autonomy level, connected channels, enabled tools), so I have full visibility into the agent's setup.
- **US-4.2.2** As a logged-in user, I want to edit my agent's system prompt, provider, model, and autonomy level from the detail page, so I can iterate on behavior without recreating the agent.
- **US-4.2.3** As a logged-in user, I want to enable or disable specific tools for my agent (shell, file, browser, memory, web search, HTTP, cron, hardware), so I can control what the agent can do.
- **US-4.2.4** As a logged-in user, I want to set rate limits (max actions per minute) and cost caps (max tokens per hour) on my agent, so I prevent runaway behavior and unexpected bills.

### 4.3 Agent Lifecycle

- **US-4.3.1** As a logged-in user, I want to start my agent with a single button click, so it begins listening on connected channels and processing requests.
- **US-4.3.2** As a logged-in user, I want to stop my agent with a single button click, so it ceases all activity and stops consuming agent-hours.
- **US-4.3.3** As a logged-in user, I want to restart my agent (stop + start), so I can apply configuration changes or recover from errors.
- **US-4.3.4** As a logged-in user, I want to delete an agent I no longer need, with a confirmation prompt, so I can free up agent slots.
- **US-4.3.5** As a logged-in user, I want to see real-time status indicators (running/stopped/errored) for each agent on the dashboard and detail pages, so I always know agent state.

### 4.4 Agent Logs

- **US-4.4.1** As a logged-in user, I want to view a chronological log of my agent's activity (messages received, tool calls made, responses sent, errors), so I can debug and monitor behavior.
- **US-4.4.2** As a logged-in user, I want to filter agent logs by type (info, warning, error) and by date range, so I can isolate specific issues.
- **US-4.4.3** As a logged-in user, I want to see tool execution details in the logs (tool name, input parameters, output/result, duration), so I can audit what tools did.

---

## Epic 5: Channel Management

### 5.1 Connect Channel

- **US-5.1.1** As a logged-in user, I want to connect a Telegram channel by providing my bot token and configuring an allowlist, so my agent can communicate via Telegram.
- **US-5.1.2** As a logged-in user, I want to connect a Discord channel by providing my bot token, guild ID, and channel ID, so my agent can communicate via Discord.
- **US-5.1.3** As a logged-in user, I want to connect a Slack channel by providing my bot token and channel name, so my agent can communicate via Slack.
- **US-5.1.4** As a logged-in user, I want to connect additional channels (WhatsApp, Matrix, Email, IRC, Mattermost, Lark, DingTalk, NextCloud Talk) through guided setup forms, so I can reach users on their preferred platform.
- **US-5.1.5** As a logged-in user, I want to see a list of all supported channels with setup instructions for each, so I know what's available and how to configure it.

### 5.2 Channel Status

- **US-5.2.1** As a logged-in user, I want to see the health status of each connected channel (healthy, degraded, disconnected), so I know if a channel needs attention.
- **US-5.2.2** As a logged-in user, I want to run a health check ("doctor") on a channel to verify connectivity and permissions, so I can troubleshoot issues.
- **US-5.2.3** As a logged-in user, I want to disconnect (remove) a channel from my agent, so I can clean up unused integrations.

### 5.3 Channel-Agent Binding

- **US-5.3.1** As a logged-in user, I want to assign one or more channels to a specific agent, so each agent only responds on its designated channels.
- **US-5.3.2** As a logged-in user, I want to reassign a channel from one agent to another, so I can reorganize without deleting and recreating.

---

## Epic 6: Memory Management

### 6.1 View Memory

- **US-6.1.1** As a logged-in user, I want to view my agent's stored memories in a searchable, paginated list, so I can see what the agent knows.
- **US-6.1.2** As a logged-in user, I want to filter memories by category (core, daily, conversation, custom), so I can find specific types of stored knowledge.
- **US-6.1.3** As a logged-in user, I want to see memory storage usage (used/limit) for my tier, so I know how close I am to my capacity.

### 6.2 Manage Memory

- **US-6.2.1** As a logged-in user, I want to manually add a memory entry (key, value, category), so I can seed my agent with specific knowledge.
- **US-6.2.2** As a logged-in user, I want to edit an existing memory entry, so I can correct or update stored knowledge.
- **US-6.2.3** As a logged-in user, I want to delete a specific memory entry, so I can remove outdated or incorrect information.
- **US-6.2.4** As a logged-in user, I want to bulk-clear memories by category or date range (with confirmation), so I can reset agent knowledge when needed.

---

## Epic 7: Billing & Subscription

### 7.1 Plan Management

- **US-7.1.1** As a logged-in user, I want to view my current plan, billing cycle dates, and next invoice amount on a billing page, so I understand my financial commitment.
- **US-7.1.2** As a logged-in user, I want to upgrade my plan (Free → Builder → Team) with a single click and immediate effect, so I can unlock more capacity when needed.
- **US-7.1.3** As a logged-in user, I want to downgrade my plan at the end of the current billing cycle, so I'm not locked into a tier I no longer need.
- **US-7.1.4** As a logged-in user, I want to start a 14-day free trial of a paid tier without entering a credit card, so I can evaluate before committing.
- **US-7.1.5** As a logged-in user on a trial, I want to see how many trial days remain on the dashboard, so I can decide whether to convert before it expires.

### 7.2 Payment

- **US-7.2.1** As a paid-tier user, I want to add a payment method (credit card via Stripe), so my subscription continues after the trial.
- **US-7.2.2** As a paid-tier user, I want to view and download past invoices, so I have records for accounting.
- **US-7.2.3** As a paid-tier user, I want to receive email notifications before my card is charged and if a payment fails, so I can keep my account in good standing.

### 7.3 Usage Tracking

- **US-7.3.1** As a logged-in user, I want to see a usage breakdown for the current billing period (agent-hours used, storage consumed, tool executions, LLM tokens via managed proxy), so I can monitor costs.
- **US-7.3.2** As a logged-in user, I want to see usage trend charts (daily/weekly) for agent-hours and storage, so I can predict upcoming costs.
- **US-7.3.3** As a logged-in user, I want to receive email alerts when I reach 80% and 100% of my tier's included limits, so I can upgrade or reduce usage before overage kicks in.

---

## Epic 8: Team Management (Team & Enterprise Tiers)

### 8.1 Team Members

- **US-8.1.1** As a Team/Enterprise admin, I want to invite team members by email, so they can access the shared workspace via their own SSO login.
- **US-8.1.2** As a Team/Enterprise admin, I want to assign roles to team members (Admin, Member, Viewer), so I can control who can modify agents, channels, and billing.
- **US-8.1.3** As a Team/Enterprise admin, I want to remove a team member, so I can revoke access when someone leaves the team.
- **US-8.1.4** As a Team/Enterprise admin, I want to see a list of all team members with their roles and last-active dates, so I can manage the team roster.
- **US-8.1.5** As a Team/Enterprise admin, I want to see how many seats are used vs. included (e.g. "5 of 5 seats used") and add additional seats at the per-seat rate, so I can grow the team.

### 8.2 Audit Trail (Team & Enterprise)

- **US-8.2.1** As a Team/Enterprise admin, I want to view an audit log of all team actions (agent created/modified/deleted, channels changed, members invited/removed, billing changes), so I have accountability and compliance records.
- **US-8.2.2** As a Team/Enterprise admin, I want to filter the audit log by user, action type, and date range, so I can investigate specific events.

---

## Epic 9: Account Settings

### 9.1 Profile

- **US-9.1.1** As a logged-in user, I want to view my profile information (name, email, avatar — pulled from SSO provider), so I can verify my identity.
- **US-9.1.2** As a logged-in user, I want to set a display name that overrides the SSO-provided name, so I can customize how I appear in the platform.
- **US-9.1.3** As a logged-in user, I want to see which SSO provider(s) are linked to my account, so I know my login options.
- **US-9.1.4** As a logged-in user, I want to link an additional SSO provider (e.g. add GitHub if I signed up with Google), so I have backup login options.

### 9.2 API Keys

- **US-9.2.1** As a logged-in user, I want to generate platform API keys for programmatic access to the SpaceClaw API, so I can integrate with CI/CD or custom tooling.
- **US-9.2.2** As a logged-in user, I want to revoke a platform API key, so I can rotate credentials or remove compromised keys.
- **US-9.2.3** As a logged-in user, I want to manage my stored LLM provider API keys (add, update, delete) in a secure vault, so my keys are encrypted at rest and I don't have to re-enter them.

### 9.3 Security

- **US-9.3.1** As a logged-in user, I want to view my active sessions and revoke any I don't recognize, so I can protect my account.
- **US-9.3.2** As an Enterprise admin, I want to configure SSO/SAML for my organization, so team members authenticate through our corporate identity provider.

### 9.4 Notifications

- **US-9.4.1** As a logged-in user, I want to configure email notification preferences (billing alerts, usage warnings, agent errors, security events), so I only receive notifications I care about.

### 9.5 Danger Zone

- **US-9.5.1** As a logged-in user, I want to delete my account (with confirmation and a grace period), so I can leave the platform if I choose.
- **US-9.5.2** As a logged-in user, I want to export my data (agent configs, memories, logs) before deleting my account, so I don't lose valuable information.

---

## Epic 10: Enterprise Features

- **US-10.1** As an Enterprise customer, I want a dedicated SpaceClaw instance (single-tenant), so my data is fully isolated from other customers.
- **US-10.2** As an Enterprise customer, I want SSO/SAML integration, so my team uses our corporate identity provider.
- **US-10.3** As an Enterprise customer, I want custom data retention policies, so I comply with my organization's regulatory requirements.
- **US-10.4** As an Enterprise customer, I want a dedicated support channel with SLA guarantees, so I get timely help when issues arise.
- **US-10.5** As an Enterprise customer, I want to access the "Contact Sales" flow from the pricing page and receive a response within 1 business day, so I can negotiate custom terms efficiently.

---

## Epic 11: Referral Program

- **US-11.1** As a logged-in user, I want to generate a unique referral link from my dashboard, so I can share it with others.
- **US-11.2** As a logged-in user, I want to receive $10 in platform credit when someone signs up using my referral link and activates a paid plan, so I'm rewarded for spreading the word.
- **US-11.3** As a referred user, I want to receive $10 in platform credit when I sign up via a referral link, so I have an incentive to try the platform.
- **US-11.4** As a logged-in user, I want to see my referral stats (link shares, signups, credits earned) on a referral dashboard, so I can track my impact.

---

## Epic 12: Marketplace (Phase 3+)

- **US-12.1** As a logged-in user, I want to browse a marketplace of community-built tools, templates, and integrations, so I can extend my agent's capabilities without building from scratch.
- **US-12.2** As a logged-in user, I want to install a marketplace item into my agent with one click, so adoption is frictionless.
- **US-12.3** As a developer, I want to publish my custom tools/templates to the marketplace, so I can share or sell my work to the SpaceClaw community.
- **US-12.4** As a marketplace publisher, I want to set a price (or free) for my listing and receive 70% of revenue, so I'm incentivized to build quality extensions.

---

## Epic 13: Responsive Design & Accessibility

- **US-13.1** As a visitor or user on a mobile device, I want the entire website (public pages and dashboard) to be fully responsive, so I can use SpaceClaw from any device.
- **US-13.2** As a user with accessibility needs, I want all pages to meet WCAG 2.1 AA standards (keyboard navigation, screen reader support, sufficient contrast), so the platform is usable by everyone.
- **US-13.3** As a user, I want pages to load in under 2 seconds on a standard connection, so the experience feels fast and professional.

---

## Epic 14: Status & Reliability

- **US-14.1** As a visitor or user, I want to access a public status page showing platform health (API, database, channels, gateway), so I can check if issues are on my end or platform-wide.
- **US-14.2** As a logged-in user, I want to subscribe to status updates via email, so I'm notified of outages and maintenance windows.

---

## Story Map Summary

| Page / Section           | Epics Covered         | Auth Required |
|--------------------------|-----------------------|---------------|
| Landing Page             | 1.1                   | No            |
| Features Page            | 1.2                   | No            |
| Pricing Page             | 1.3                   | No            |
| Docs (external link)     | 1.4                   | No            |
| Login Page               | 2.1                   | No            |
| Dashboard                | 3.1, 4.3, 11         | Yes           |
| Agent Create / Detail    | 4.1, 4.2, 4.3, 4.4   | Yes           |
| Channel Management       | 5.1, 5.2, 5.3        | Yes           |
| Memory Management        | 6.1, 6.2             | Yes           |
| Billing & Usage          | 7.1, 7.2, 7.3        | Yes           |
| Team Management          | 8.1, 8.2             | Yes (Admin)   |
| Account Settings         | 9.1–9.5              | Yes           |
| Marketplace              | 12                    | Yes           |
| Status Page              | 14                    | No            |

---

*This document is the source of truth for website page generation. Each user story maps to UI components, API endpoints, and acceptance criteria to be defined during implementation.*
