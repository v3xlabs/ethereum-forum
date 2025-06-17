use chrono::{DateTime, Utc};
use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, query, query_as, query_scalar};

use crate::{
    models::{discourse::topic::DiscourseTopicPost, topics::POSTS_PER_PAGE},
    state::AppState,
};

#[derive(Debug, Serialize, Deserialize, FromRow, Object)]
pub struct Post {
    pub discourse_id: String,
    pub post_id: i32,
    pub topic_id: i32,
    pub user_id: i32,
    pub post_number: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cooked: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkshopPost {
    pub discourse_id: String,
    pub post_id: i32,
    pub user_id: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cooked: Option<String>,
}

impl From<Post> for WorkshopPost {
    fn from(post: Post) -> WorkshopPost {
        Self {
            discourse_id: post.discourse_id,
            post_id: post.post_id,
            user_id: post.user_id,
            updated_at: post.updated_at,
            created_at: post.created_at,
            cooked: post.cooked,
            // get username field from extra object
            username: None,
            // username: self.extra.map(|x| x.as_object().map(|y| y.get("username"))).flatten(),
        }
    }
}

impl Post {
    pub fn from_discourse(discourse_id: &str, post: DiscourseTopicPost) -> Self {
        let mut extra = post.extra.clone();
        let extra = extra.as_object_mut().unwrap();
        extra.insert("username".to_string(), post.username.into());
        let extra = serde_json::to_value(extra).unwrap();

        Self {
            discourse_id: discourse_id.to_string(),
            post_id: post.id,
            topic_id: post.topic_id,
            user_id: post.user_id,
            post_number: post.post_number,
            updated_at: Some(post.updated_at),
            created_at: Some(post.created_at),
            cooked: Some(post.cooked),
            post_url: post.post_url,
            extra: Some(extra),
        }
    }

    pub async fn upsert(&self, state: &AppState) -> Result<(), sqlx::Error> {
        query!("INSERT INTO posts (discourse_id, post_id, topic_id, user_id, post_number, updated_at, cooked, post_url, extra) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) ON CONFLICT (discourse_id, post_id) DO UPDATE SET discourse_id=$1, post_id=$2, topic_id=$3, user_id=$4, post_number=$5, updated_at = $6, cooked = $7, post_url = $8, extra = $9",
            self.discourse_id,
            self.post_id,
            self.topic_id,
            self.user_id,
            self.post_number,
            self.updated_at,
            self.cooked,
            self.post_url,
            self.extra,
        )
        .execute(&state.database.pool)
        .await?;
        Ok(())
    }

    pub async fn find_by_topic_id(
        discourse_id: &str,
        topic_id: i32,
        page: i32,
        size: Option<i32>,
        state: &AppState,
    ) -> Result<(Vec<Self>, bool), sqlx::Error> {
        let size = size.unwrap_or(POSTS_PER_PAGE as i32);
        let offset = (page - 1) * size;
        let posts = query_as!(
            Self,
            "SELECT * FROM posts WHERE discourse_id = $1 AND topic_id = $2 ORDER BY post_number ASC LIMIT $3 OFFSET $4",
            discourse_id,
            topic_id,
            (size + 1) as i64,
            offset as i64
        )
        .fetch_all(&state.database.pool)
        .await?;

        let has_more = posts.len() == size as usize + 1;
        let posts = posts.into_iter().take(size as usize).collect();

        Ok((posts, has_more))
    }

    pub async fn count_by_topic_id(
        discourse_id: &str,
        topic_id: i32,
        state: &AppState,
    ) -> Result<i32, sqlx::Error> {
        let count = query_scalar!(
            "SELECT COUNT(*) FROM posts WHERE discourse_id = $1 AND topic_id = $2",
            discourse_id,
            topic_id
        )
        .fetch_one(&state.database.pool)
        .await?;

        Ok(count.unwrap_or_default() as i32)
    }
}
