# LLM Harness

How the LLM features work today, their exact limits, and the planned evolution.

## Current state

Two features, both **stateless, single-shot, zero-tool completions** against any
OpenAI-compatible API (`LLM_API_KEY`, `LLM_BASE_URL`, `LLM_MODEL`, optional
`LLM_DIGEST_MODEL`). Code lives in `app/src/modules/llm/`.

### Execution model

| Property | Value |
|---|---|
| Turns per run | Exactly 1 (no agentic loop, no retries at harness level) |
| Tool access | None (`tools` is never sent; the MCP client was removed) |
| Memory / state | None — every run starts from scratch |
| Max completion tokens | 16,000 (both features; reasoning models spend these on thinking first) |
| Input budget | ~180k estimated tokens (3.5 chars/token heuristic, `tokens.rs`) |
| Empty output | Treated as failure; never cached |

### Per-topic summary (`summary.rs`)

- Input: the **first 512 posts, top-down** (`ORDER BY post_number ASC`), full
  `cooked` HTML + username + timestamps, serialized as **one JSON user message**
  with the topic metadata.
- Streamed via SSE to the first requester; concurrent requesters share the same
  upstream call (coalesced replay); completed streams linger 30s.
- Cached in `topic_summaries`; a row is fresh while `based_on == last_post_at`.
  The cache is **serve-only** — a regeneration never sees the previous summary.
- Cannot read other threads, other threads' summaries, or anything else.

Known caveats (inherited from the original implementation):

1. **Threads > 512 posts lose the bottom** — the newest posts are cut off.
2. **Truncation is message-granular.** The whole thread is one message, so a
   payload over ~630KB is dropped *entirely*, leaving only the system prompt —
   the model would summarize nothing. Reachable for 512 posts averaging >1.2KB
   of HTML.

### Homepage activity digest (`digest.rs`)

- Every 12h (anchored on the latest row's `created_at`; restarts don't
  re-generate early; failures retry after 1h; `POST /api/admin/digest` forces).
- Input: 12 topics (top `view_count` among those active in the last 3 days),
  each with its **8 newest posts from those 3 days** in chronological order as
  `strip_tags`'d excerpts capped at 800 chars.
- Reads across threads, but only that recent slice — no full histories, no
  topic summaries, no previous digests.
- Cached in `activity_digests`, served by `GET /api/digest`.

## Recommended evolution

Three phases, each independently shippable. The unifying primitive is
**"prior state + delta"**: never re-read everything, always fold new
information into an existing artifact.

### Phase 1 — incremental summaries (fixes full-thread coverage)

Replace the flat 512-post payload with an incremental fold:

- Store `based_on_post_number` alongside `based_on` in `topic_summaries`.
- **Update run**: input = previous summary + posts since
  `based_on_post_number`, prompt = "revise this summary given the new posts."
- **Cold start / oversized delta**: chunk posts into ~N-token segments and fold
  each chunk through the same update prompt (chunk 1 → summary v0, + chunk 2 →
  v1, …). Same code path as updates; guarantees every post is read exactly once
  over the lifetime of a thread, with strictly bounded input per call.
- Trim **within** the payload (drop oldest posts in the chunk, never the
  message) and make the budget explicit: `LLM_MAX_INPUT_TOKENS` (default well
  under real context sizes, e.g. 60–100k).

#### Output format: evergreen overview + code-owned timeline

Incremental generation must never leak "since the previous summary, X changed"
phrasing into what readers see. The summary becomes a **structured JSON
artifact** instead of one markdown blob:

```jsonc
{
  "overview": "…",            // timeless, high-level markdown; fully rewritten
                              // each run; prompt forbids meta-references
                              // ("recently", "since last time", "new posts…")
  "key_points": ["…"],        // optional; stable bullets for visualization
  "open_questions": ["…"],    // optional
  "changelog_entry": {        // the ONLY delta-shaped field the model returns
    "period_start": "…", "period_end": "…",
    "post_range": [113, 141],
    "entry": "Markdown describing what changed in this window"
  }
}
```

Key decision — **the timeline is owned by code, not the model**: each run the
model returns the updated `overview` plus a single `changelog_entry` for the
new window; the app *appends* that entry to a `timeline` array stored in the
DB row. The model never re-emits (and therefore can never rewrite or lose) the
history. Cap the stored timeline (e.g. last 20 entries; older ones are the
librarian's problem in Phase 3).

- Storage: `summary_json JSONB` on `topic_summaries` (keep `summary_text` as
  the rendered-markdown fallback during migration).
- Frontend: renders `overview` as the main body, `timeline` as a collapsed
  "What's changed" section at the bottom; the structured fields (`key_points`,
  `post_range`, timestamps) are what make richer visualization possible later.
- Enforcement: request JSON output (`response_format: json_object` where the
  server supports it, else parse + validate, one retry on invalid). Empty or
  schema-invalid output = failure, never cached.
- Streaming: keep streaming raw tokens; the frontend progressively renders the
  `overview` field via tolerant incremental JSON parsing and reveals the rest
  on completion. (Fallback if that's fiddly: show progress during generation,
  render on completion — the coalesced/cached path is unaffected.)
- The digest adopts the same shape (`overview` + per-topic `highlights` with
  topic refs + its own code-owned run-over-run timeline), which also gives the
  homepage section structured data instead of freeform markdown.

### Phase 2 — bounded tool loop

Re-introduce tool calling as a small **in-process, bounded executor** (direct
function calls to the same code the MCP server uses — no HTTP hop, no remote
MCP client):

- Tools: `get_topic_summary(discourse_id, topic_id)`,
  `get_topic_overview` (metadata + first post + stats),
  `get_posts(topic, from, to)` (hard-capped page size),
  `search_forum(query)`, and `resolve_links` (internal links found in the
  current thread → topic refs).
- **Summary-first results**: a tool call about another thread returns its
  cached summary when fresh, else an overview — never hundreds of raw posts.
  This is what keeps "read other threads" affordable.
- Hard limits enforced by the executor, not the prompt: max tool calls per run
  (~8), max round trips (~4), shared input-token budget across the whole loop,
  per-result token cap. Loop ends when the model answers without tool calls or
  a limit trips (then: force a final answer with tools disabled).

### Phase 3 — shared system memory ("lexicon")

A curated glossary of terms, definitions, and accumulated context, injected
into every run — without the model ever mentioning it.

- Table `llm_memory`: `entry_id`, `term`, `content`, `updated_at`, `sources`
  (topic refs), plus a versioned snapshot so it can be rolled back. Total
  rendered size hard-capped (~4k tokens).
- **Injection**: rendered into the system prompt as a neutral "Background
  context and terminology" section, with an instruction to use it silently and
  never reference the section itself in output.
- **Writes — two-tier, single curator**:
  1. Generation runs get one cheap `note_candidate(term, content)` tool that
     appends to a *staging* table. Candidates are never injected.
  2. A periodic **librarian run** (e.g. daily, after the digest) reads the
     current memory + staged candidates + recent summaries, and emits the
     consolidated next memory version: merge duplicates, resolve conflicts,
     drop stale entries, keep under the size cap.
  Single-writer consolidation prevents noise, drift, and prompt-injection-ish
  garbage from any one run from persisting unreviewed; staging keeps capture
  cheap. The librarian's diff is loggable/reviewable, and bad versions roll
  back by pointer.
- The digest run should also receive the previous digest, making "what's
  new/changed" literal rather than inferred.

### Cross-cutting

- All limits live in `LlmConfig` (env-overridable, sane defaults): input
  budget, completion cap, tool-call/turn caps, per-result cap, memory cap.
- Persist per-run metadata (tokens in/out, tool calls, duration, outcome) to a
  small `llm_runs` table for tuning and cost visibility.

## Tracing (Langfuse)

`llm_runs` stays the source of truth: the curator reads recent runs as prompt
input, and the admin metrics endpoints aggregate over it. Langfuse sits
alongside it as an export target, linked by `llm_runs.langfuse_trace_id`.

What Langfuse adds over the admin run viewer: the actual prompts and
completions (deliberately never written to `llm_runs.trace`, which stays
payload-free), USD cost, cross-run dashboards, and sessions that group every
run for a thread.

### Running it

```
docker compose -f app/compose.langfuse.yml up -d
```

UI on <http://localhost:3100>. Ports differ from upstream Langfuse because the
app binds 3000 and the base stack binds 5432, so `langfuse-web` moves to 3100
and its Postgres to 5433. The `LANGFUSE_INIT_*` variables bootstrap the org,
project, user and API keys, so a fresh volume needs no manual UI step.

**The credentials in `compose.langfuse.yml` are local-development defaults.**
`SALT`, `ENCRYPTION_KEY`, `NEXTAUTH_SECRET`, `REDIS_AUTH`, and the MinIO and
ClickHouse passwords must all be overridden before deploying anywhere
reachable.

### Wiring

Set `LANGFUSE_HOST`, `LANGFUSE_PUBLIC_KEY` and `LANGFUSE_SECRET_KEY` (see
`.env.example`). Leaving them unset disables tracing entirely and skips prompt
capture, exactly like `LlmService` degrades without `LLM_API_KEY`.

Spans are built retroactively in `modules::telemetry` from `RunRecorder`'s
recorded timings, so tracing adds no control flow to the LLM paths. Transport
is OTLP over HTTP — Langfuse does not accept gRPC. Every OpenTelemetry crate
must stay on one version (currently 0.29, the version poem requires); a split
graph means two `global` statics and silently dropped telemetry.

Cost stays blank until a model definition matching the configured model name is
added in the Langfuse UI, since built-in pricing is matched by regex against
provider-style names.

### Payload shapes the UI depends on

Langfuse derives tool definitions and tool calls by *parsing* the captured
input and output — no attribute carries them. Three constraints follow, and
breaking any of them silently degrades the UI to raw text rather than erroring:

- Captured input must be `{"messages": [...], "tools": [...]}`. A bare message
  array still renders as a conversation but yields no tool definitions.
- A tool-calling round must capture the **whole** response message, not its
  `content` — content is null exactly when the model calls tools. `tool_calls`
  must keep the nested `{function: {name, arguments}}` form with `arguments` as
  a string; a flat form with object arguments fails Langfuse's schema and drops
  the entire conversation view.
- Truncation happens per string *inside* the payload
  (`RunRecorder::capture_value`). Truncating the serialized JSON leaves it
  unparseable, which is exactly the silent-fallback case above.

Tool executions are emitted as `langfuse.observation.type = "tool"` with
`gen_ai.tool.name` and `gen_ai.tool.call.id`, which gives them the wrench icon
and tool filtering.

### What we send the models

Discourse `cooked` is rendered HTML, and it carries structure the summarizer
needs: code blocks, tables, links, and quoted posts with attribution. It is
converted to Markdown (`modules::llm::content`), **not** stripped — stripping
is ~6pp smaller but discards code fences, link targets and quote attribution.
Only rendering chrome is dropped: icon SVGs, avatars, and upload dimension
labels.

Posts reach the model as text, not JSON:

```
topic: <title> (<discourse_id>/<topic_id>)

#<post_number> @<username> <YYYY-MM-DD>
<markdown>
```

The discourse instance is stated once per chunk instead of once per post, and
`post_id`/`user_id`/`updated_at` are not sent at all — the model cannot use a
raw numeric id, and `summary.md` asks it to cite `#p-<post_number>` and
`@username`, which are the fields it now actually receives.

The glossary is filtered to entries whose term appears in the text being
summarized (`relevant_memory`), with the token budget still the hard ceiling.
Injecting all of it made every call scale with the size of the glossary rather
than with the work being done. The curator passes `None` and keeps receiving
everything, since it is the thing that edits the glossary.

Measured on `magicians/75` (a 111-post thread), before and after: **110,620 →
47,564 input tokens, −57%**, and 3 fold sections became 2 because more posts fit
per chunk.

### Traces, sessions and nesting

A trace id is unique per run and is generated in `RunRecorder::new`, not at
export time, so work a run triggers can attach to it while the run is still
going.

Sessions are one per invocation:

| Run | Session | Groups |
|---|---|---|
| summary | `summary:{discourse_id}:{topic_id}@{trace}` | one summarization attempt |
| digest | `digest@{trace}` | one digest run |
| curator | `curator@{trace}` | one curator run |

One session per invocation, so the sessions list reads as a list of attempts.
The descriptor prefix keeps it scannable; the trace suffix keeps repeat
attempts distinct.

A summary that a digest or curator triggers through `get_topic_summary` is
emitted **inside the caller's trace**, nested under the specific tool-call
span, so the whole cascade reads as one timeline. Two consequences to know:

- `llm_runs` is no longer 1:1 with traces — every run in the cascade shares the
  parent's `langfuse_trace_id`, so the admin deep link opens the parent trace
  (which contains the nested spans).
- A nested summary does not appear under its own topic session; it lives in the
  caller's.

Nesting only applies when the caller actually *started* the run. A run that
coalesced onto an in-flight generation keeps its own trace, because a run
cannot belong to two traces.

### Why a regeneration may produce no trace

Two guards short-circuit before any LLM work, and both are correct. The
`status` field in the start-summary response says which fired:

- `existing` — a fresh cached summary was returned. Regeneration needs
  `?force=true` (a **query** parameter, plus the admin key), not a body field.
- `ongoing` — coalesced onto an in-flight run.

Only `started` produces a new run, and therefore a new trace.
