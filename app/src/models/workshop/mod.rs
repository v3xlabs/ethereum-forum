use sqlx::{query_as, types::Uuid};
use chrono::{DateTime, Utc};
use crate::state::AppState;

#[derive(Debug, sqlx::FromRow)]
pub struct WorkshopMessage {
    pub message_id: Uuid,
    pub chat_id: Uuid,
    pub sender_role: String, // "user", "assistant", or "system"
    pub message: String,
    pub created_at: DateTime<Utc>,
    pub parent_message_id: Option<Uuid>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct WorkshopChat {
    pub chat_id: Uuid,
    pub user_id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub summary: Option<String>,
    pub last_message_id: Option<Uuid>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct WorkshopSnapshot {
    pub snapshot_id: Uuid,
    pub chat_id: Uuid,
    pub user_id: i32,
    pub message_id: Uuid,
    pub created_at: DateTime<Utc>,
}

impl WorkshopChat {
    pub async fn find_by_user_id(user_id: i32, state: &AppState) -> Result<Vec<Self>, sqlx::Error> {
        query_as("SELECT * FROM workshop_chats WHERE user_id = $1")
            .bind(user_id)
            .fetch_all(&state.database.pool)
            .await
    }

    pub async fn create(user_id: i32, state: &AppState) -> Result<Self, sqlx::Error> {
        query_as("INSERT INTO workshop_chats (user_id) VALUES ($1) RETURNING *")
            .bind(user_id)
            .fetch_one(&state.database.pool)
            .await
    }
}

impl WorkshopMessage {
    pub async fn create_user_message(chat_id: Option<Uuid>, parent_message_id: Option<Uuid>, user_id: i32, message: String, state: &AppState) -> Result<Self, sqlx::Error> {
        let chat_id = match chat_id {
            Some(chat_id) => chat_id,
            _ => {
                let chat = WorkshopChat::create(user_id, state).await?;
                chat.chat_id
            }
        };

        query_as("INSERT INTO workshop_messages (chat_id, user_id, sender_role, message, parent_message_id) VALUES ($1, $2, $3, $4, $5) RETURNING *")
            .bind(chat_id)
            .bind(user_id)
            .bind("user")
            .bind(message)
            .bind(parent_message_id)
            .fetch_one(&state.database.pool)
            .await
    }

    pub async fn get_messages_by_chat_id(chat_id: Uuid, state: &AppState) -> Result<Vec<Self>, sqlx::Error> {
        query_as("SELECT * FROM workshop_messages WHERE chat_id = $1")
            .bind(chat_id)
            .fetch_all(&state.database.pool)
            .await
    }
}
