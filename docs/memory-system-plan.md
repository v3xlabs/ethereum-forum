# Memory System Improvement Plan

Status: **in progress** — Tranches A and D implemented; frontend admin improvements (usage chart, in-progress runs, tool call traces) done. Next up is Tranche B (per-user memories).
Last updated: 2026-07-21

This document captures the full assessment of the current LLM memory system
for ethereum-forum and the planned improvements, broken into independently
shippable tranches. It is the source of truth for what we are building and why;
adjust the plan here before changing code.

## Context

ethereum-forum is a data aggregator tracking the Ethereum Magicians
(ethereum-magicians.org) and Ethereum Research (ethresear.ch) Discourse forums.
Backend: Rust (poem + sqlx + Postgres + Meilisearch + async-openai). Frontend:
React 19 + TanStack Router. The LLM harness lives in `app/src/modules/llm/`
with three agents — summarizer, digest, curator — sharing a bounded tool-loop
executor and a shared "memory" glossary injected into every run.

Relevant files:
- `app/src/modules/llm/prompts/{summary,digest,curator}.md` — agent prompts
- `app/src/modules/llm/{summary,digest,curator,executor,tokens,recorder,streams,mod}.rs` — agent runtimes
- `app/src/models/llm/mod.rs` — `LlmMemory`, `LlmMemorySnapshot`, `LlmRun`
- `app/src/models/topics/post.rs` — `Post`, `SummaryPost`
- `app/src/server/admin.rs` — admin endpoints (memory CRUD, curator trigger)
- `web/src/routes/u/$discourseId/$userId/index.tsx` — user profile page
- `app/migrations/0015..0019` — llm_runs, llm_memory, snapshots
- `docs/llm.md` — original design doc (Phase 3 = shared memory)

## Assessment of the current memory system

Note: the local dev Postgres is at migration 14 only — `llm_memory`,
`llm_runs`, `summary_json`, and snapshots do not exist there. Production has
not rolled these features out yet, so there are no live memories to dump. The
judgment below is structural, based on prompts, schema, and code paths.

### Design-level problems

1. **No time in the curator's world.** `curator.md` and the payload built in
   `curator.rs::run_curator_inner` never inject the current date. Worse, the
   prompt *bans* temporal language ("no 'recently', 'currently under
   discussion'") while the product goal is the opposite — dated facts. The
   model is obeying the prompt; the prompt is wrong for the goal. This is the
   root cause of entries like "works at consensys" (undated) and "mentioned in
   recent discussions" (vague).

2. **Post edit dates are invisible to the model.** The `GetPosts` tool
   (`executor.rs` builtin) returns `created_at` but drops `updated_at`. Posts
   are upserted with both timestamps (`post.rs::Post::upsert`), and the DB
   carries `updated_at`, but the tool that the curator and summarizer use to
   verify claims cannot see edits. The model therefore cannot date-stamp "as
   of edit on …".

3. **Sources are rendered with Debug formatting.** In
   `curator.rs::run_curator_inner`, `memory_text` is built with
   `format!("... (sources: {:?})", m.sources)`. The model sees raw
   `{"url":...,"reason":...}` JSON Debug output rather than clean labeled
   links, making provenance hard to reason about during curation.

4. **Sources are never injected into summarizers.**
   `build_shared_memory_section` (duplicated in `summary.rs` and `digest.rs`)
   renders only `term: content`. The links the curator carefully curates are
   invisible to every summarizer/digest run, so they cannot be verified or
   surfaced downstream.

5. **`memory_token_budget` is configured but not enforced.** `LlmConfig`
   declares `memory_token_budget` (default 4096), but
   `build_shared_memory_section` renders all entries with no cap. A growing
   glossary silently bloats every run. Additionally the serde default is
   `#[serde(default)]` which yields `0` when `LLM_MEMORY_TOKEN_BUDGET` is
   unset via the `LLM_` env prefix — the `Default` impl's 4096 is bypassed
   when loading from env. This is a latent bug: if the field is absent from
   env, budget becomes 0.

6. **`note_candidate` writes straight to `llm_memory`.** Phase 3's staging
   tier was dropped (migration `0017_drop_staging`). Any summarizer or digest
   run can pollute the live glossary instantly; the curator only cleans up
   ~24h later. This is the most likely source of low-quality entries reaching
   summarizer prompts.

7. **Curator toolset is thinner than the summarizer's.** Curator has
   `search_forum`, `get_topic_summary`, `get_posts` — but not
   `get_topic_overview`, and nothing user-scoped. It cannot easily verify a
   person-specific claim ("works at Consensys") by reading that person's
   posts.

8. **No GitHub tooling.** The PM module regex-extracts
   `github.com/ethereum/pm/issues/N` from topics, and curator URL
   normalization allows `ethereum/EIPs` and `ethereum/ERCs` as source links,
   but there is no tool to actually read issue/PR/comment content. Meeting
   threads reference `ethereum/pm` issues heavily; the model has no way to
   enrich summaries with that context.

### Content-level symptoms (from user report)

- "mentioned in recent discussions" — vague + undated; exactly what the
  prompt's anti-temporal rule produces when the model has no date to anchor
  to.
- "works at consensys" — undated biographical fact; the prompt gives no
  instruction to date-stamp such claims and no date input to use.

Both are fixed structurally by Tranche A, not by prompt nudging.

## Decisions (from user)

| Question | Decision |
|---|---|
| Starting point | **A first, then B and C** (A is the foundation) |
| Per-user memory keying | Accept username **or** `discourse_id` from the model; resolve username → numeric Discourse user_id when validating. Key the table on `(discourse_id, user_id)` numeric, snapshot username for display. |
| Restore staging table (Tranche D) | **Yes** — `note_candidate` writes to staging; only curator promotes to `llm_memory`. |
| GitHub auth | **Unauthenticated only.** 60 req/h shared. Acceptable because curator runs daily and tool calls are bounded. No `GITHUB_TOKEN` env handling. |
| Live memories | Local DB is stale (migration 14); nothing to dump. Proceed from design analysis. |

## Tranche A — Time-awareness foundation

**Goal:** give the curator (and summarizer) accurate time context so memory
entries and perspectives carry real dates instead of vague language. This is
the prerequisite for B and C — without it, per-user memories and GitHub
verification would reproduce the same undated-fact problem.

**Scope:** curator prompt + payload, `GetPosts` tool, source rendering, memory
budget enforcement, shared memory renderer.

### A1. Rewrite `app/src/modules/llm/prompts/curator.md`

- State that the current date is injected into the system prompt.
- Split the quality bar into two kinds:
  - **Definitions** (protocols, EIPs, mechanisms, jargon): evergreen, one to
    three sentences, NO temporal language.
  - **Attributed / biographical / relational facts** (people, affiliations,
    stances, who proposed what): time-bound, MUST carry `(as of YYYY-MM-DD)`
    using the date verified against. Give good/bad examples:
    - Good: "Works at Consensys (as of 2026-07-12)."
    - Good: "Proposed EIP-7702 (as of 2024-03-05)."
    - Bad: "Works at Consensys." (undated)
    - Bad: "Mentioned in recent discussions." (vague + undated)
- Instruct the curator to verify time-bound claims by fetching the cited post
  with `get_posts`, which returns both `created_at` and `updated_at`. Prefer
  `updated_at` when the claim depends on an edit, else `created_at`.
- Rewrite source-label guidance: keep reasons to a few words ("core
  proposal", "specification", "vitalik's post", "consensys bio"), not full
  sentences.
- Add a "Tool use" section encouraging targeted verification before
  adding/correcting/removing a time-bound fact.

### A2. Inject current date into curator system prompt

`app/src/modules/llm/curator.rs::run_curator_inner`:

- Compute `let now = chrono::Utc::now();` at the start.
- Build the system message content as
  `format!("{CURATOR_PROMPT}\n\n## Current date\n\n{now}", now = now.format("%Y-%m-%d %H:%M UTC"))`.
- Apply to both the tool-loop system message and the final-completion system
  message (the `system_prompt` variable that wraps `CURATOR_PROMPT`).

### A3. Render memory sources cleanly in the curator payload

`curator.rs::run_curator_inner` — replace the `memory_text` map:

```
format!("- {}: {} (sources: {:?})", m.term, m.content, m.sources)
```

with a renderer that emits each source as `url — reason` (or just `url` when
reason is empty). For example:

```
- EIP-1559: Ethereum fee market change... (sources:
  - /t/magicians/1234#p-2 — core proposal
  - https://eips.ethereum.org/EIPS/eip-1559 — specification)
```

This makes provenance legible to the model during curation.

### A4. Add `updated_at` to `GetPosts` tool output

`app/src/modules/llm/executor.rs` — `builtin::GetPosts::call` currently emits
`post_number`, `user_id`, `created_at`, `excerpt`. Add `updated_at` alongside
`created_at` in the JSON object so the curator (and summarizer) can see when a
post was last edited.

### A5. Add `updated_at` to the summarizer post payload (optional, low-risk)

`app/src/models/topics/post.rs::SummaryPost` already carries `updated_at` and
`created_at`, and `Post::from_discourse` populates both. The summarizer
serializes `SummaryPost` into the user message via `serde_json::to_string`, so
`updated_at` is already in the payload **provided** `Post::from` for
`SummaryPost` keeps it — which it does. No code change needed here unless we
want to ensure `username` is populated (it's currently hardcoded `None` — a
pre-existing TODO, out of scope for A).

### A6. Enforce `memory_token_budget` and fix its serde default

`app/src/modules/llm/mod.rs`:

- Fix the latent bug: `memory_token_budget` has `#[serde(default)]` which
  yields `0` when loaded via `Figment::new().merge(Env::prefixed("LLM_"))`.
  Add a `default_memory_token_budget() -> usize { 4096 }` function and use
  `#[serde(default = "default_memory_token_budget")]`, matching the pattern
  used by `max_input_tokens` etc.

`app/src/modules/llm/{summary,digest}.rs`:

- Extract a single shared renderer (see A7) and have it cap rendered entries
  to `memory_token_budget` tokens (using `estimate_tokens_in_text`). When the
  budget is exceeded, drop the lowest-value entries (e.g. shortest content, or
  simply stop appending once the budget is reached — entries are already
  `ORDER BY term ASC`; a deterministic cutoff is fine for v1). Log a warning
  when truncation occurs.

### A7. Shared memory renderer with source labels

Add a module-level function in `app/src/modules/llm/mod.rs` (or a new
`memory.rs`):

```
pub fn render_memory_section(memory: &[LlmMemory], token_budget: usize) -> String
```

- Renders each entry as `- **{term}**: {content}` followed by indented source
  lines `  - {url} — {reason}` (reason omitted if empty/None).
- Enforces `token_budget` via `estimate_tokens_in_text`.
- Returns the full `## Background context and terminology\n\n...\n\nUse the
  above context silently and never reference this section in your output.`
  wrapper, or empty string when memory is empty or budget is 0.

Replace the two duplicated `build_shared_memory_section` implementations in
`summary.rs` and `digest.rs` with calls to this shared renderer. This gives
summarizers visibility into source links (fixes problem #4) and enforces the
budget in one place (fixes #5).

### A8. Verification

- `cargo check` and `cargo test` in `app/` — done. Existing curator tests pass;
  added `modules::llm::tests::{empty_memory_or_zero_budget_returns_empty,
  renders_term_content_and_labelled_sources, budget_cap_drops_entries}` for the
  shared renderer's budget cap and source labelling.
- Manual: trigger the curator via `POST /admin/llm/curator/trigger` and
  inspect the resulting `llm_memory` rows for dated biographical entries.
  **Pending — user to run against a migrated environment.**

### Files touched in A

- `app/src/modules/llm/prompts/curator.md` (rewrite)
- `app/src/modules/llm/curator.rs` (date injection, clean source rendering)
- `app/src/modules/llm/executor.rs` (`GetPosts` + `updated_at`)
- `app/src/modules/llm/mod.rs` (fix serde default; add `render_memory_section`)
- `app/src/modules/llm/summary.rs` (use shared renderer)
- `app/src/modules/llm/digest.rs` (use shared renderer)

## Tranche B — Per-user memories

**Goal:** extend perspectives to carry user ids, persist per-user memories
curated by the curator, and surface them admin-only on the `/u/` page. Depends
on A (so per-user facts are dated and verifiable).

### B1. Schema: `llm_user_memory` table

New migration `app/migrations/0020_llm_user_memory.sql`:

```sql
CREATE TABLE llm_user_memory (
    entry_id SERIAL PRIMARY KEY,
    discourse_id TEXT NOT NULL,
    user_id INT NOT NULL,             -- numeric Discourse user id
    username TEXT NOT NULL,           -- snapshot at write time; display only
    content TEXT NOT NULL,            -- dated biographical/relational facts
    sources JSONB DEFAULT '[]'::jsonb,-- [{url, reason}]
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (discourse_id, user_id)
);

CREATE INDEX idx_llm_user_memory_user ON llm_user_memory(discourse_id, user_id);
```

Key on `(discourse_id, user_id)` numeric because usernames are mutable.
`username` is a display snapshot refreshed on each curator write.

### B2. Model + persistence

`app/src/models/llm/mod.rs` — add `LlmUserMemory` struct (mirror `LlmMemory`
plus `discourse_id`, `user_id`, `username`) with:
- `get_by_user(discourse_id, user_id, state) -> Vec<LlmUserMemory>`
- `upsert(discourse_id, user_id, username, content, sources, state)`
- `delete_by_user(discourse_id, user_id, state) -> bool`

### B3. Extend perspectives to include `user_id`

`app/src/modules/llm/prompts/summary.md` — the `perspectives[].people[]`
object gains a `user_id` field (numeric Discourse id). Update the schema
example and instruct the model to include the numeric id when known (it
appears on each post as `user_id`). Username remains for display/linking.

Frontend types in `web/src/components/topics/StreamingSummary.tsx`
(`PerspectivePerson`) gain an optional `user_id?: number`. The username link
to `/u/$discourseId/$userId` is unchanged (route still uses username as path
param).

### B4. Curator per-user memory lane

`app/src/modules/llm/curator.rs`:

- Add a user-resolution helper: given a model-provided identifier that may be
  a username or a `discourse_id`/numeric id, resolve to a numeric
  `(discourse_id, user_id, username)`. Use the existing Discourse user-fetch
  cache (`DiscourseService::fetch_discourse_user_cached`) to resolve usernames
  → ids. If resolution fails, skip the update and log in `action_log`.
- Extend `CuratorOutput` with:
  - `user_memory_updates: Vec<UserMemoryUpdate>` where `UserMemoryUpdate` is
    `{discourse_id, user_id?, username?, content, sources}`. The curator
    accepts either `user_id` or `username`; the resolver fills the other.
  - `user_memory_removals: Vec<UserMemoryRemoval>` where `UserMemoryRemoval`
    is `{discourse_id, user_id?, username?}`.
- Apply `user_memory_updates` via `LlmUserMemory::upsert` (after resolving),
  and `user_memory_removals` via `LlmUserMemory::delete_by_user`. Count both
  in the run `metadata`.
- Update `curator.md` output format to document the two new arrays, with the
  same dated-fact discipline from A1 applied to per-user content.

### B5. Curator user-scoped verification tool

Port the MCP server's `search_by_user` / `get_user_summary` logic (already in
`app/src/server/mcp.rs`) into an in-process `LlmTool` so the curator can read
a user's recent posts to verify person-specific claims. Add to the curator
toolset in `curator.rs::make_tool_defs` and `tool_impls`.

Tool: `get_user_posts(discourse_id, username_or_id, limit)` — returns recent
posts by that user with `created_at`/`updated_at` and excerpts, so the curator
can confirm "works at Consensys (as of …)" by reading the user's own
self-introduction post.

### B6. Admin-only surface on `/u/` page

Backend: `app/src/server/admin.rs` — add
`GET /admin/llm/user-memory?discourse_id=&user_id=` (admin-key gated)
returning `Vec<LlmUserMemory>` for that user.

Frontend: `web/src/routes/u/$discourseId/$userId/index.tsx` — add a
"Agent notes" section rendered **only** when an admin token is present (reuse
existing admin auth pattern). Fetches from the new admin endpoint and renders
each entry's `content` with its source links. Non-admins see nothing (no
layout shift, no placeholder).

Note: the `/u/` route's `$userId` param is currently the **username**, not the
numeric id. The admin endpoint must accept username and resolve it, OR the
frontend must pass the numeric id obtained from `useUser`'s
`userData.user.id`. Prefer the latter to avoid double resolution.

### B7. Verification

- `cargo test` (add tests for user-resolution helper and the new curator
  output parsing).
- Manual: trigger curator, confirm `llm_user_memory` rows appear with dated
  content; visit `/u/magicians/<username>/` as admin and see the section.

### Files touched in B

- `app/migrations/0020_llm_user_memory.sql` (new)
- `app/src/models/llm/mod.rs` (`LlmUserMemory`)
- `app/src/modules/llm/curator.rs` (user lane, resolver, new tool)
- `app/src/modules/llm/prompts/curator.md` (output format)
- `app/src/modules/llm/prompts/summary.md` (perspectives `user_id`)
- `app/src/modules/llm/executor.rs` (`GetUserPosts` tool)
- `app/src/server/admin.rs` (`GET /admin/llm/user-memory`)
- `web/src/components/topics/StreamingSummary.tsx` (`PerspectivePerson.user_id`)
- `web/src/routes/u/$discourseId/$userId/index.tsx` (admin "Agent notes" section)
- `web/src/api/` (new admin user-memory hook)

## Tranche C — GitHub readonly tools

**Goal:** let the curator (and optionally the summarizer) read
issues/PRs/comments from `ethereum/eips`, `ethereum/ercs`, and `ethereum/pm`
to enrich summaries of meeting threads and verify EIP/ERC references. Depends
on A (dated context) and complements B (per-user facts about contributors).

### C1. Toolset (in-process `LlmTool` implementations)

New module `app/src/modules/llm/github.rs` with these tools, all backed by
`reqwest` against `api.github.com` **unauthenticated** (60 req/h shared —
acceptable given bounded tool calls and daily curator cadence):

- `get_github_issue(owner, repo, number)` — body, state, labels,
  `created_at`/`updated_at`/`closed_at`, author login.
- `get_github_issue_comments(owner, repo, number, limit)` — paginated
  comments with author, body excerpt, `created_at`/`updated_at`.
- `get_github_pr(owner, repo, number)` — PR metadata, state, merged status,
  dates, author.
- `get_github_pr_reviews(owner, repo, number)` — review states + reviewers.
- `get_github_pr_comments(owner, repo, number)` — review comments.

All responses are JSON-trimmed to the fields the model needs (no raw
paginated envelopes).

### C2. Structural repo allowlist (enforced in code, not prompt)

Every tool validates `owner == "ethereum"` and
`repo ∈ {"eips", "ercs", "pm"}` before issuing any request. Mismatched
inputs return an error string. This makes the "limited to those three repos"
guarantee structural — the model cannot escape it via prompt injection.

### C3. Wire into the curator

`app/src/modules/llm/curator.rs::make_tool_defs` and `tool_impls` — add the
GitHub tools. Update `curator.md` to mention them: use
`get_github_issue`/`get_github_pr` to verify EIP/ERC references and to pull
context from `ethereum/pm` meeting issues when a forum thread references a
meeting.

### C4. Extend source-link normalization

`app/src/modules/llm/curator.rs::normalize_memory_url` — add
`https://github.com/ethereum/pm/...` to the allowed prefixes (eips/ercs
already allowed). This lets the curator attach `ethereum/pm` issue links as
memory sources.

### C5. Optional: wire into summarizer

If meeting-thread summaries benefit, add the GitHub tools to the summarizer
toolset in `summary.rs::make_tool_defs` as well. Keep this optional and
gated on observed need — the curator is the primary consumer.

### C6. Rate-limit awareness

Because unauthenticated GitHub is 60 req/h shared across the whole process,
the executor's existing `max_tool_calls` (8) and `max_tool_rounds` (6) caps
already bound a single run. Add a small in-process token-bucket or a simple
"last request" min-interval guard in `github.rs` to avoid bursts. Log a
warning (and return a clear error to the model) when the rate limit is hit,
so the curator can fall back to forum-only verification.

### C7. Verification

- `cargo test` (add tests for the repo allowlist: assert non-ethereum owners
  and non-allowlisted repos are rejected).
- Manual: trigger curator on a topic that references an `ethereum/pm` issue
  and confirm the issue content appears in the curator's tool trace
  (`llm_runs.trace`).

### Files touched in C

- `app/src/modules/llm/github.rs` (new)
- `app/src/modules/llm/mod.rs` (declare `github` module)
- `app/src/modules/llm/curator.rs` (toolset + `normalize_memory_url` pm prefix)
- `app/src/modules/llm/prompts/curator.md` (mention GitHub tools)
- `app/src/modules/llm/summary.rs` (optional toolset addition)

## Tranche D — Restore curation hygiene (staging)

**Goal:** stop summarizer/digest runs from writing directly to the live
glossary. Reintroduce the staging tier from the original Phase 3 design so
only the curator promotes entries to `llm_memory`. This was approved by the
user and should land alongside or shortly after A (it is independent of B/C).

### D1. Schema: restore `llm_memory_staging`

New migration `app/migrations/0021_restore_staging.sql`:

```sql
CREATE TABLE llm_memory_staging (
    staging_id SERIAL PRIMARY KEY,
    term TEXT NOT NULL,
    content TEXT NOT NULL,
    source_discourse_id TEXT,
    source_topic_id INT,
    source_post_number INT,
    link_reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_llm_memory_staging_created ON llm_memory_staging(created_at DESC);
```

(Migration `0017` dropped the old staging table; this recreates it with the
extra `link_reason` column matching the current `note_candidate` signature.)

### D2. `note_candidate` writes to staging

`app/src/modules/llm/executor.rs::builtin::NoteCandidate::call` — change the
write target from `LlmMemory::upsert` to a new
`LlmMemoryStaging::insert(...)`. Staging rows are never injected into
summarizer prompts.

### D3. Curator promotes from staging

`curator.rs::run_curator_inner`:
- Load recent staging rows (`LlmMemoryStaging::recent(limit)`) and include
  them in the payload as a "Staged candidates" section.
- The curator's `memory_updates` already upserts to `llm_memory`; add logic
  to clear promoted staging rows (e.g. delete staged rows whose `term` was
  upserted this run, or clear all rows older than the curator run timestamp).
- Update `curator.md` to describe the staging flow: staged candidates are
  suggestions for the curator to evaluate and promote (or reject).

### D4. Model + admin surface

- `LlmMemoryStaging` model in `app/src/models/llm/mod.rs` with `insert`,
  `recent`, `delete_by_term`, `clear_before(timestamp)`.
- Optional admin endpoint `GET /admin/llm/memory/staging` for visibility into
  pending candidates.

### D5. Verification

- `cargo test` — done, 16 tests pass.
- Manual: run a summarizer, confirm a `note_candidate` call creates a staging
  row but does NOT appear in `llm_memory`; trigger curator, confirm the
  candidate is promoted (or rejected) and staging is cleared.
  **Pending — user to run against a migrated environment.**

### Files touched in D

- `app/migrations/0020_restore_staging.sql` (new)
- `app/src/models/llm/mod.rs` (`LlmMemoryStaging`)
- `app/src/modules/llm/executor.rs` (`NoteCandidate` → staging)
- `app/src/modules/llm/curator.rs` (load + promote + clear staging)
- `app/src/modules/llm/prompts/curator.md` (staging flow description)
- `app/src/modules/llm/prompts/{summary,digest}.md` (note_candidate desc)
- `app/src/server/admin.rs` (`GET /admin/llm/memory/staging`)

## Implementation order

1. **Tranche A** (time-awareness) — ✅ implemented.
2. **Tranche D** (staging) — ✅ implemented. Restores
   glossary hygiene before per-user memories multiply the write surface.
3. **Tranche B** (per-user memories) — depends on A; builds on D's discipline.
4. **Tranche C** (GitHub tools) — depends on A; independent of B. Can be done
   in parallel with B if desired.

## Out of scope / deferred

- Rollback API for snapshots (`LlmMemorySnapshot::rollback_to` exists but no
  endpoint exposes it). Not needed for A–D.
- Enforcing `memory_token_budget` on the per-user memory injection (B) —
  per-user memories are not injected into every run, only surfaced on `/u/`,
  so budget pressure is lower. Revisit if we later inject per-user context
  into summarizers.
- Authenticated GitHub (token support) — explicitly deferred per user
  decision; unauthenticated 60 req/h is accepted for now.
- The pre-existing `SummaryPost.username` TODO (hardcoded `None`) — not
  blocking A, but worth fixing separately so perspectives can link users
  reliably.
