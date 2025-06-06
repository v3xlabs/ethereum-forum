use poem::IntoEndpoint;
use poem_mcpserver::{
    McpServer, Tools,
    content::{Json, Text},
    streamable_http,
};

use crate::{
    models::topics::{Post, Topic},
    state::AppState,
};

pub struct ForumTools {
    state: AppState,
}

impl ForumTools {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[Tools]
impl ForumTools {
    /// Get the latest summary for a topic.
    async fn get_topic_summary(&self, topic_id: i32) -> Text<String> {
        match Topic::get_summary_by_topic_id(topic_id, &self.state).await {
            Ok(summary) => Text(summary.summary_text),
            Err(err) => Text(format!("error: {err}")),
        }
    }

    /// Get posts for a topic. Page starts at 1.
    async fn get_posts(
        &self,
        topic_id: i32,
        page: Option<i32>,
        size: Option<i32>,
    ) -> Json<Vec<Post>> {
        let page = page.unwrap_or(1);
        match Post::find_by_topic_id(topic_id, page, size, &self.state).await {
            Ok((posts, _)) => Json(posts),
            Err(err) => Json(vec![Post {
                post_id: -1,
                topic_id,
                user_id: 0,
                post_number: 0,
                updated_at: None,
                created_at: None,
                cooked: Some(format!("error: {err}")),
                post_url: None,
                extra: None,
            }]),
        }
    }
}

pub fn endpoint(state: AppState) -> impl IntoEndpoint {
    streamable_http::endpoint(move |_| McpServer::new().tools(ForumTools::new(state.clone())))
}
