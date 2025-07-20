use chrono::{DateTime, Utc};
use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, query_as};
use uuid::Uuid;

use crate::{models::workshop::message::WorkshopMessage, state::AppState};

#[derive(Debug, FromRow, Serialize, Deserialize, Object)]
pub struct WorkshopSnapshot {
    pub snapshot_id: Uuid,
    pub chat_id: Uuid,
    pub user_id: Uuid,
    pub message_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Serialize, Object)]
pub struct CreateChatSnapshotPayload {
    pub chat_id: Uuid,
    pub message_id: Uuid,
}

#[derive(Debug, Deserialize, Serialize, Object)]
pub struct WorkshopSnapshotResponse {
    pub snapshot: WorkshopSnapshot,
    pub messages: Vec<WorkshopMessage>
}

impl WorkshopSnapshot {
    pub async fn create(chat_id: Uuid, message_id: Uuid, user_id: Uuid, state: &AppState) -> Result<Self, sqlx::Error> {
        sqlx::query!(
            "SELECT chat_id FROM workshop_chats WHERE chat_id = $1 AND deleted_at IS NULL",
            chat_id
        )
        .fetch_one(&state.database.pool)
        .await?;

        let snapshot = query_as!(WorkshopSnapshot, 
            "INSERT INTO workshop_snapshots (chat_id, message_id, user_id) VALUES ($1, $2, $3) RETURNING *", 
            chat_id, message_id, user_id)
            .fetch_one(&state.database.pool)
            .await?;
        Ok(snapshot)
    }

    pub async fn get_by_snapshot_id(snapshot_id: Uuid, state: &AppState) -> Result<Self, sqlx::Error> {
        let snapshot = query_as!(WorkshopSnapshot, 
            "SELECT s.snapshot_id, s.chat_id, s.user_id, s.message_id, s.created_at 
             FROM workshop_snapshots s 
             INNER JOIN workshop_chats c ON s.chat_id = c.chat_id 
             WHERE s.snapshot_id = $1 AND c.deleted_at IS NULL", 
            snapshot_id)
            .fetch_one(&state.database.pool)
            .await?;
        Ok(snapshot)
    }
}

impl WorkshopSnapshotResponse {
    pub async fn get_snapshot_response(snapshot_id: Uuid, state: &AppState) -> Result<Self, sqlx::Error> {
        let snapshot = WorkshopSnapshot::get_by_snapshot_id(snapshot_id, state).await?;
        let messages = WorkshopMessage::get_messages_by_chat_id(&snapshot.chat_id, state).await?;
        Ok(Self {
            snapshot,
            messages,
        })
    }
}
