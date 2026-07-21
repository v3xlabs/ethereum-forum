use chrono::{DateTime, Utc};
use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow, Object, Clone)]
pub struct LlmRun {
    pub run_id: Uuid,
    pub run_type: String,
    pub discourse_id: Option<String>,
    pub topic_id: Option<i32>,
    pub prompt_tokens: Option<i32>,
    pub completion_tokens: Option<i32>,
    pub total_tokens: Option<i32>,
    pub reasoning_tokens: Option<i32>,
    pub model_used: Option<String>,
    pub tool_calls: Option<i32>,
    pub tool_rounds: Option<i32>,
    pub duration_ms: Option<i32>,
    pub outcome: String,
    pub error: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub trace: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Default)]
pub struct LlmRunDraft {
    pub run_type: &'static str,
    pub discourse_id: Option<String>,
    pub topic_id: Option<i32>,
    pub prompt_tokens: Option<i32>,
    pub completion_tokens: Option<i32>,
    pub reasoning_tokens: Option<i32>,
    pub model_used: Option<String>,
    pub tool_calls: Option<i32>,
    pub tool_rounds: Option<i32>,
    pub duration_ms: i32,
    pub outcome: &'static str,
    pub error: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub trace: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Object, Clone)]
pub struct LlmMemory {
    pub entry_id: i32,
    pub term: String,
    pub content: String,
    /// Array of `MemoryLink` objects: where this term's definition comes from.
    pub sources: Option<serde_json::Value>,
    pub updated_at: DateTime<Utc>,
}

/// A source link attached to a memory entry: a site-relative URL (e.g.
/// `/t/magicians/1234#p-56`) plus why that link matters ("the core idea",
/// "vitalik's opinion").
#[derive(Debug, Serialize, Deserialize, Object, Clone)]
pub struct MemoryLink {
    pub url: String,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Object, Clone)]
pub struct LlmMemorySnapshot {
    pub snapshot_id: i32,
    pub version: i32,
    pub memory_snapshot: serde_json::Value,
    pub curator_run_id: Option<Uuid>,
    pub summary: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// A staged glossary candidate written by a summarizer/digest run via
/// `note_candidate`. The curator is the only thing that promotes these into
/// `llm_memory`, so summarizer prompts are never polluted by unverified
/// candidates.
#[derive(Debug, Serialize, Deserialize, FromRow, Object, Clone)]
pub struct LlmMemoryStaging {
    pub staging_id: i32,
    pub term: String,
    pub content: String,
    pub source_discourse_id: Option<String>,
    pub source_topic_id: Option<i32>,
    pub source_post_number: Option<i32>,
    pub link_reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl LlmRun {
    pub async fn insert(
        draft: LlmRunDraft,
        state: &crate::state::AppState,
    ) -> Result<Self, sqlx::Error> {
        let total_tokens = match (draft.prompt_tokens, draft.completion_tokens) {
            (None, None) => None,
            (prompt, completion) => Some(prompt.unwrap_or(0) + completion.unwrap_or(0)),
        };
        sqlx::query_as!(
            Self,
            r#"INSERT INTO llm_runs
               (run_type, discourse_id, topic_id, prompt_tokens, completion_tokens, total_tokens,
                reasoning_tokens, model_used, tool_calls, tool_rounds, duration_ms, outcome, error, metadata, trace)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
               RETURNING run_id, run_type, discourse_id, topic_id, prompt_tokens, completion_tokens,
                         total_tokens, reasoning_tokens, model_used, tool_calls, tool_rounds,
                         duration_ms, outcome, error, metadata, trace, created_at"#,
            draft.run_type,
            draft.discourse_id.as_deref(),
            draft.topic_id,
            draft.prompt_tokens,
            draft.completion_tokens,
            total_tokens,
            draft.reasoning_tokens,
            draft.model_used.as_deref(),
            draft.tool_calls,
            draft.tool_rounds,
            draft.duration_ms,
            draft.outcome,
            draft.error.as_deref(),
            draft.metadata,
            draft.trace
        )
        .fetch_one(&state.database.pool)
        .await
    }

    pub async fn delete_latest_digest(state: &crate::state::AppState) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM activity_digests WHERE digest_id = (SELECT digest_id FROM activity_digests ORDER BY created_at DESC LIMIT 1)")
            .execute(&state.database.pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// Inserts a placeholder row with `outcome = 'running'` at the start of a
    /// run so the admin UI can show in-flight work. Call `update_outcome`
    /// with the final stats when the run finishes.
    pub async fn insert_running(
        run_type: &str,
        discourse_id: Option<&str>,
        topic_id: Option<i32>,
        state: &crate::state::AppState,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(
            Self,
            r#"INSERT INTO llm_runs
               (run_type, discourse_id, topic_id, outcome)
               VALUES ($1, $2, $3, 'running')
               RETURNING run_id, run_type, discourse_id, topic_id, prompt_tokens, completion_tokens,
                          total_tokens, reasoning_tokens, model_used, tool_calls, tool_rounds,
                          duration_ms, outcome, error, metadata, trace, created_at"#,
            run_type,
            discourse_id,
            topic_id,
        )
        .fetch_one(&state.database.pool)
        .await
    }

    /// Updates a running row with final stats. Sets `outcome` from the draft.
    pub async fn update_outcome(
        run_id: Uuid,
        draft: LlmRunDraft,
        state: &crate::state::AppState,
    ) -> Result<(), sqlx::Error> {
        let total_tokens = match (draft.prompt_tokens, draft.completion_tokens) {
            (None, None) => None,
            (prompt, completion) => Some(prompt.unwrap_or(0) + completion.unwrap_or(0)),
        };
        sqlx::query!(
            r#"UPDATE llm_runs
               SET prompt_tokens = $2, completion_tokens = $3, total_tokens = $4,
                   reasoning_tokens = $5, model_used = $6, tool_calls = $7, tool_rounds = $8,
                   duration_ms = $9, outcome = $10, error = $11, metadata = $12, trace = $13
               WHERE run_id = $1"#,
            run_id,
            draft.prompt_tokens,
            draft.completion_tokens,
            total_tokens,
            draft.reasoning_tokens,
            draft.model_used.as_deref(),
            draft.tool_calls,
            draft.tool_rounds,
            draft.duration_ms,
            draft.outcome,
            draft.error.as_deref(),
            draft.metadata,
            draft.trace,
        )
        .execute(&state.database.pool)
        .await?;
        Ok(())
    }

    /// Updates a running row if one was created, otherwise falls back to
    /// inserting a fresh row.
    pub async fn finalize(
        run_id: Option<Uuid>,
        draft: LlmRunDraft,
        state: &crate::state::AppState,
    ) {
        match run_id {
            Some(id) => {
                if let Err(e) = Self::update_outcome(id, draft, state).await {
                    tracing::error!("failed to update running llm_run {id}: {e}");
                }
            }
            None => {
                let _ = Self::insert(draft, state).await;
            }
        }
    }
}

impl LlmMemory {
    pub async fn get_all(state: &crate::state::AppState) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(Self, "SELECT * FROM llm_memory ORDER BY term ASC")
            .fetch_all(&state.database.pool)
            .await
    }

    pub async fn upsert(
        term: &str,
        content: &str,
        sources: &serde_json::Value,
        state: &crate::state::AppState,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(
            Self,
            r#"INSERT INTO llm_memory (term, content, sources)
               VALUES ($1, $2, $3)
               ON CONFLICT (term) DO UPDATE SET content = $2, sources = $3, updated_at = NOW()
               RETURNING entry_id, term, content, sources, updated_at"#,
            term,
            content,
            sources
        )
        .fetch_one(&state.database.pool)
        .await
    }

    pub async fn delete(entry_id: i32, state: &crate::state::AppState) -> Result<bool, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM llm_memory WHERE entry_id = $1", entry_id)
            .execute(&state.database.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn delete_by_term(
        term: &str,
        state: &crate::state::AppState,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM llm_memory WHERE term = $1", term)
            .execute(&state.database.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}

impl LlmMemoryStaging {
    pub async fn insert(
        term: &str,
        content: &str,
        source_discourse_id: Option<&str>,
        source_topic_id: Option<i32>,
        source_post_number: Option<i32>,
        link_reason: Option<&str>,
        state: &crate::state::AppState,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(
            Self,
            r#"INSERT INTO llm_memory_staging
               (term, content, source_discourse_id, source_topic_id, source_post_number, link_reason)
               VALUES ($1, $2, $3, $4, $5, $6)
               RETURNING staging_id, term, content, source_discourse_id, source_topic_id,
                         source_post_number, link_reason, created_at"#,
            term,
            content,
            source_discourse_id,
            source_topic_id,
            source_post_number,
            link_reason
        )
        .fetch_one(&state.database.pool)
        .await
    }

    pub async fn recent(
        limit: i64,
        state: &crate::state::AppState,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            Self,
            "SELECT staging_id, term, content, source_discourse_id, source_topic_id,
                    source_post_number, link_reason, created_at
             FROM llm_memory_staging
             ORDER BY created_at DESC
             LIMIT $1",
            limit
        )
        .fetch_all(&state.database.pool)
        .await
    }

    pub async fn delete_by_term(
        term: &str,
        state: &crate::state::AppState,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM llm_memory_staging WHERE term = $1", term)
            .execute(&state.database.pool)
            .await?;
        Ok(result.rows_affected())
    }

    pub async fn clear_before(
        cutoff: DateTime<Utc>,
        state: &crate::state::AppState,
    ) -> Result<u64, sqlx::Error> {
        let result =
            sqlx::query!("DELETE FROM llm_memory_staging WHERE created_at <= $1", cutoff)
                .execute(&state.database.pool)
                .await?;
        Ok(result.rows_affected())
    }
}

impl LlmMemorySnapshot {
    pub async fn get_latest(state: &crate::state::AppState) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Self,
            "SELECT * FROM llm_memory_snapshots ORDER BY version DESC LIMIT 1"
        )
        .fetch_optional(&state.database.pool)
        .await
    }

    pub async fn rollback_to(
        snapshot_id: i32,
        state: &crate::state::AppState,
    ) -> Result<bool, sqlx::Error> {
        let snapshot = sqlx::query_as!(
            Self,
            "SELECT * FROM llm_memory_snapshots WHERE snapshot_id = $1",
            snapshot_id
        )
        .fetch_optional(&state.database.pool)
        .await?
        .ok_or(sqlx::Error::Protocol("snapshot not found".into()))?;

        let entries: Vec<serde_json::Value> = serde_json::from_value(snapshot.memory_snapshot).unwrap_or_default();
        for entry in entries {
            let term = entry["term"].as_str().unwrap_or("");
            let content = entry["content"].as_str().unwrap_or("");
            let sources = entry.get("sources").cloned().unwrap_or(serde_json::json!([]));
            if term.is_empty() {
                continue;
            }
            Self::upsert_entry(term, content, &sources, state).await?;
        }
        Ok(true)
    }

    async fn upsert_entry(
        term: &str,
        content: &str,
        sources: &serde_json::Value,
        state: &crate::state::AppState,
    ) -> Result<(), sqlx::Error> {
        LlmMemory::upsert(term, content, sources, state).await?;
        Ok(())
    }

    pub async fn create(
        version: i32,
        memory_snapshot: &serde_json::Value,
        curator_run_id: Option<Uuid>,
        summary: Option<&str>,
        state: &crate::state::AppState,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(
            Self,
            r#"INSERT INTO llm_memory_snapshots (version, memory_snapshot, curator_run_id, summary)
               VALUES ($1, $2, $3, $4)
               RETURNING snapshot_id, version, memory_snapshot, curator_run_id, summary, created_at"#,
            version,
            memory_snapshot,
            curator_run_id,
            summary
        )
        .fetch_one(&state.database.pool)
        .await
    }
}
