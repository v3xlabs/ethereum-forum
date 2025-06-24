use chrono::{DateTime, Utc};
use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, query_as};
use uuid::Uuid;

use crate::state::AppState;

#[derive(Debug, FromRow, Serialize, Deserialize, Object)]
pub struct WorkshopChat {
    pub chat_id: Uuid,
    pub user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_message_id: Option<Uuid>,
}

impl WorkshopChat {
    pub async fn find_by_user_id(user_id: Uuid, state: &AppState) -> Result<Vec<Self>, sqlx::Error> {
        query_as("SELECT * FROM workshop_chats WHERE user_id = $1 ORDER BY created_at DESC")
            .bind(user_id)
            .fetch_all(&state.database.pool)
            .await
    }

    pub async fn find_by_id(chat_id: Uuid, state: &AppState) -> Result<Self, sqlx::Error> {
        query_as("SELECT * FROM workshop_chats WHERE chat_id = $1")
            .bind(chat_id)
            .fetch_one(&state.database.pool)
            .await
    }

    pub async fn create(user_id: Uuid, state: &AppState) -> Result<Self, sqlx::Error> {
        query_as("INSERT INTO workshop_chats (user_id) VALUES ($1) RETURNING *")
            .bind(user_id)
            .fetch_one(&state.database.pool)
            .await
    }

    pub async fn update_last_message(chat_id: &Uuid, message_id: &Uuid, state: &AppState) -> Result<Self, sqlx::Error> {
        query_as("UPDATE workshop_chats SET last_message_id = $1 WHERE chat_id = $2 RETURNING *")
            .bind(message_id)
            .bind(chat_id)
            .fetch_one(&state.database.pool)
            .await
    }

    pub async fn update_summary(chat_id: &Uuid, summary: &str, state: &AppState) -> Result<Self, sqlx::Error> {
        query_as("UPDATE workshop_chats SET summary = $1 WHERE chat_id = $2 RETURNING *")
            .bind(summary)
            .bind(chat_id)
            .fetch_one(&state.database.pool)
            .await
    }
}

