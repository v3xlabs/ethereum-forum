use openai::chat::{ChatCompletion, ChatCompletionMessage, ChatCompletionMessageRole};
use serde_json::json;

use crate::{
    models::topics::{Post, Topic},
    state::AppState,
};

#[derive(serde::Serialize)]
pub struct Workbench;

impl Workbench {
    pub async fn create_workshop_summary(topic: &Topic, state: &AppState) -> String {
        let posts = Post::find_by_topic_id(topic.topic_id, 1, Some(50), state);

        let messages = vec![
            state.workshop.prompts.summerize.clone(),
            ChatCompletionMessage {
                role: ChatCompletionMessageRole::User,
                content: Some(
                    serde_json::to_string(&json!({
                        "topic_info": topic,
                        "posts": posts.await.map(|(posts, _)| posts).unwrap_or_else(|_| vec![]),
                    }))
                    .unwrap(),
                ),
                name: None,
                function_call: None,
                tool_call_id: None,
                tool_calls: None,
            },
        ];

        let chat_completion =
            ChatCompletion::builder("meta-llama/llama-3.3-8b-instruct:free", messages.clone())
                .credentials(state.workshop.crendentials.clone())
                .create()
                .await
                .unwrap();
        let response = chat_completion.choices.first().unwrap().message.clone();
        return response.content.unwrap().trim().to_string();
    }
}
