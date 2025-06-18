use chrono::{DateTime, Utc};
use octocrab::models::issues::Issue as OctocrabIssue;
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
    pub user: serde_json::Value, // JSONB field for user info
    pub labels: String,
    pub locked: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl GitHubIssue {
    pub fn from_octocrab(repository_url: &str, issue: &OctocrabIssue) -> Self {
        let labels_json = serde_json::to_string(&issue.labels).unwrap_or_else(|_| "[]".to_string());
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

    /// Upsert the GitHub issue into the database
    pub async fn upsert(&self, state: &AppState) -> Result<(), sqlx::Error> {
        query!(
            r#"
            INSERT INTO github_issues (repository_url, id, number, title, state, "user", labels, locked, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (id) DO UPDATE SET
                repository_url = $1,
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

    pub async fn get_by_number(number: i32, state: &AppState) -> Result<Option<Self>, sqlx::Error> {
        let issue = sqlx::query_as!(
            Self,
            r#"SELECT repository_url, id, number, title, state, "user", labels, locked, created_at, updated_at FROM github_issues WHERE number = $1"#,
            number
        )
        .fetch_optional(&state.database.pool)
        .await?;

        Ok(issue)
    }
}
