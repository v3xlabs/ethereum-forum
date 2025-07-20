use chrono::{DateTime, Utc};
use octocrab::models::issues::{Comment as OctocrabComment, Issue as OctocrabIssue};
use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, query};

use crate::state::AppState;

#[derive(Debug, Serialize, Deserialize, FromRow, Object, Clone)]
pub struct GitHubIssue {
    pub repository_url: String,
    pub id: String,
    pub number: i32,
    pub title: String,
    pub state: String,
    pub user: serde_json::Value,   // JSONB field for user info
    pub labels: serde_json::Value, // JSONB field for labels array
    pub locked: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Object, Clone)]
pub struct GitHubIssueComment {
    pub repository_url: String,
    pub issue_id: String,
    pub id: String,
    pub user: serde_json::Value,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl GitHubIssue {
    pub fn from_octocrab(repository_url: &str, issue: &OctocrabIssue) -> Self {
        let labels_json = serde_json::to_value(&issue.labels)
            .unwrap_or_else(|_| serde_json::Value::Array(vec![]));
        let user_json = serde_json::to_value(&issue.user).unwrap_or(serde_json::Value::Null);

        Self {
            repository_url: repository_url.to_string(),
            id: issue.id.to_string(),
            number: issue.number as i32,
            title: issue.title.clone(),
            state: format!("{:?}", issue.state),
            user: user_json,
            labels: labels_json,
            locked: issue.locked,
            created_at: issue.created_at,
            updated_at: issue.updated_at,
        }
    }

    pub async fn upsert(&self, state: &AppState) -> Result<(), sqlx::Error> {
        query!(
            r#"
            INSERT INTO github_issues (repository_url, id, number, title, state, "user", labels, locked, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (repository_url, id) DO UPDATE SET
            number = $3,
            title = $4,
            state = $5,
            "user" = $6,
            labels = $7,
            locked = $8,
            created_at = $9,
            updated_at = $10
            "#,
            self.repository_url,
            self.id,
            self.number,
            self.title,
            self.state,
            self.user,
            self.labels,
            self.locked,
            self.created_at,
            self.updated_at,
        )
        .execute(&state.database.pool)
        .await?;

        Ok(())
    }

    pub async fn get_by_number(
        repository_url: &str,
        number: i32,
        state: &AppState,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            GitHubIssue,
            r#"SELECT repository_url, id, number, title, state, "user", labels, locked, created_at, updated_at 
               FROM github_issues 
               WHERE repository_url = $1 AND number = $2"#,
            repository_url,
            number
        )
        .fetch_optional(&state.database.pool)
        .await
    }

    pub async fn get_id_by_number(
        repository_url: &str,
        number: i32,
        state: &AppState,
    ) -> Result<Option<String>, sqlx::Error> {
        let issue_id = sqlx::query_scalar!(
            "SELECT id FROM github_issues WHERE repository_url = $1 AND number = $2",
            repository_url,
            number
        )
        .fetch_optional(&state.database.pool)
        .await?;

        Ok(issue_id)
    }
}

impl GitHubIssueComment {
    pub fn from_octocrab(repository_url: &str, issue_id: &str, comment: &OctocrabComment) -> Self {
        let user_json = serde_json::to_value(&comment.user).unwrap_or(serde_json::Value::Null);

        Self {
            repository_url: repository_url.to_string(),
            issue_id: issue_id.to_string(),
            id: comment.id.to_string(),
            user: user_json,
            body: comment.body.as_deref().unwrap_or("").to_string(),
            created_at: comment.created_at,
            updated_at: comment.updated_at.unwrap_or(comment.created_at),
        }
    }

    pub async fn upsert(&self, state: &AppState) -> Result<(), sqlx::Error> {
        query!(
            r#"
            INSERT INTO github_issue_comments (repository_url, issue_id, id, "user", body, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (repository_url, issue_id, id) DO UPDATE SET
                "user" = $4,
                body = $5,
                created_at = $6,
                updated_at = $7
            "#,
            self.repository_url,
            self.issue_id,
            self.id,
            self.user,
            self.body,
            self.created_at,
            self.updated_at
        )
        .execute(&state.database.pool)
        .await?;

        Ok(())
    }
}
