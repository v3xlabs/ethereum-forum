use chrono::{DateTime, Utc};
use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, query_as};

use crate::state::AppState;

#[derive(Debug, Serialize, Deserialize, FromRow, Object, Clone)]
pub struct ActivityDigest {
    pub digest_id: i32,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub digest_text: String,
    pub topics_included: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

impl ActivityDigest {
    pub async fn get_latest(state: &AppState) -> Result<Option<Self>, sqlx::Error> {
        query_as!(
            Self,
            "SELECT * FROM activity_digests ORDER BY created_at DESC LIMIT 1"
        )
        .fetch_optional(&state.database.pool)
        .await
    }

    pub async fn insert(
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
        digest_text: &str,
        topics_included: serde_json::Value,
        state: &AppState,
    ) -> Result<Self, sqlx::Error> {
        query_as!(
            Self,
            "INSERT INTO activity_digests (period_start, period_end, digest_text, topics_included) VALUES ($1, $2, $3, $4) RETURNING *",
            period_start,
            period_end,
            digest_text,
            topics_included
        )
        .fetch_one(&state.database.pool)
        .await
    }
}
