use discourse_webhooks::{
    PostWebhookEvent, TopicWebhookEvent, WebhookError, WebhookEventHandler, WebhookProcessor,
    async_trait,
};
use poem::{Result, web::Data};
use poem_openapi::param::Header;
use poem_openapi::{Object, OpenApi};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::models::topics::Topic;
use crate::models::topics::post::Post;
use crate::server::ApiTags;
use crate::state::AppState;

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct WebhookPayload {
    #[serde(default)]
    pub event: Option<String>,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
    #[serde(default)]
    pub timestamp: Option<i64>,
}
struct DiscourseEventHandler {
    instance: String,
    state: AppState,
}

impl DiscourseEventHandler {
    fn new(instance: String, state: AppState) -> Self {
        Self { instance, state }
    }

    async fn upsert_topic_from_event(
        &mut self,
        event: &TopicWebhookEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let topic = Topic {
            discourse_id: self.instance.clone(),
            topic_id: event.topic.id,
            title: event.topic.title.clone(),
            slug: event.topic.slug.clone(),
            post_count: event.topic.posts_count,
            view_count: event.topic.views,
            like_count: event.topic.like_count,
            image_url: None, // WebhookTopic doesn't have image_url field
            created_at: event.topic.created_at,
            last_post_at: Some(event.topic.last_posted_at),
            bumped_at: None, // WebhookTopic doesn't have bumped_at field
            pm_issue: None,
            extra: None,
        };

        let upsert_result = topic.upsert(&self.state).await;
        let instance = self.instance.clone();
        let enqueue_result = self
            .state
            .discourse
            .enqueue(instance.as_str(), event.topic.id, 1)
            .await;

        if let Err(e) = upsert_result {
            info!("Error processing topic upsert: {:?}", e);
            return Err("Failed to process topic upsert".into());
        }
        if let Err(e) = enqueue_result {
            info!("Error enqueuing topic: {:?}", e);
            return Err("Failed to enqueue topic".into());
        }
        Ok(())
    }

    async fn upsert_post_from_event(
        &mut self,
        event: &PostWebhookEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let post = Post {
            discourse_id: self.instance.clone(),
            post_id: event.post.id,
            topic_id: event.post.topic_id,
            user_id: event.post.user_id,
            post_number: event.post.post_number,
            updated_at: Some(event.post.updated_at),
            created_at: Some(event.post.created_at),
            cooked: Some(event.post.cooked.clone()),
            post_url: Some(event.post.post_url.clone()),
            extra: None,
        };

        let posts_per_page = 20; // Discourse fetches posts in pages of 20 by default
        let upsert_result = post
            .upsert(&self.state)
            .await
            .map_err(|e| anyhow::anyhow!(e));
        let instance = self.instance.clone();
        let page = ((event.post.post_number.max(1) - 1) / posts_per_page) + 1;
        let enqueue_result = self
            .state
            .discourse
            .enqueue(instance.as_str(), event.post.topic_id, page as u32)
            .await
            .map_err(|e| anyhow::anyhow!(e));

        if let Err(e) = upsert_result {
            info!("Error processing post upsert: {:?}", e);
            return Err("Failed to process post upsert".into());
        }
        if let Err(e) = enqueue_result {
            info!("Error enqueuing post: {:?}", e);
            return Err("Failed to enqueue post".into());
        }
        Ok(())
    }
}

#[async_trait]
impl WebhookEventHandler for DiscourseEventHandler {
    type Error = Box<dyn std::error::Error + Send + Sync>;
    async fn handle_topic_created(&mut self, event: &TopicWebhookEvent) -> Result<(), Self::Error> {
        self.upsert_topic_from_event(event).await
    }

    async fn handle_topic_edited(&mut self, event: &TopicWebhookEvent) -> Result<(), Self::Error> {
        self.upsert_topic_from_event(event).await
    }

    async fn handle_post_created(&mut self, event: &PostWebhookEvent) -> Result<(), Self::Error> {
        self.upsert_post_from_event(event).await
    }

    async fn handle_post_edited(&mut self, event: &PostWebhookEvent) -> Result<(), Self::Error> {
        self.upsert_post_from_event(event).await
    }
}

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct WebhookApi;

#[OpenApi]
impl WebhookApi {
    /// /webhook/discourse
    ///
    /// Handle Discourse webhook events
    #[oai(
        path = "/webhook/discourse",
        method = "post",
        tag = "ApiTags::Webhooks"
    )]
    async fn discourse_webhook(
        &self,
        state: Data<&AppState>,
        // Oddly enough we have to use Binary here because otherwise Poem won't accept JSON body
        body: poem_openapi::payload::Binary<Vec<u8>>,
        #[oai(name = "X-Discourse-Instance")] discourse_id: Header<String>,
        #[oai(name = "X-Discourse-Event")] discourse_event: Header<String>,
        #[oai(name = "X-Discourse-Event-Signature")] signature: Header<String>,
    ) -> Result<poem_openapi::payload::PlainText<String>> {
        let secret = std::env::var("DISCOURSE_WEBHOOK_SECRET");

        if secret.is_err() {
            return Err(poem::Error::from_string(
                "DISCOURSE_WEBHOOK_SECRET environment variable is not set",
                poem::http::StatusCode::INTERNAL_SERVER_ERROR,
            ));
        }
        let processor = WebhookProcessor::new().with_secret(secret.as_ref().unwrap());

        let instance = match discourse_id.0.as_str() {
            "http://localhost" => "magicians",
            "https://ethereum-magicians.org" => "magicians",
            _ => {
                return Err(poem::Error::from_string(
                    "Invalid Discourse instance",
                    poem::http::StatusCode::FORBIDDEN,
                ));
            }
        };

        let discourse_event = discourse_event.0;

        let mut handler = DiscourseEventHandler::new(instance.to_string(), state.0.clone());

        let body_str = String::from_utf8_lossy(&body.0);

        match processor
            .process(
                &mut handler,
                discourse_event.as_str(),
                &body_str,
                Some(signature.0.as_str()),
            )
            .await
        {
            Ok(_) => Ok(poem_openapi::payload::PlainText(
                "Webhook processed successfully".to_string(),
            )),
            Err(WebhookError::InvalidSignature) => Err(poem::Error::from_string(
                "Read the code at https://github.com/v3xlabs/ethereum-forum/blob/master/app/src/server/webhooks/mod.rs before trying that again :)",
                poem::http::StatusCode::FORBIDDEN,
            )),
            Err(e) => {
                println!("Error processing webhook: {:?}", e);
                Err(poem::Error::from_string(
                    format!("Error processing webhook"),
                    poem::http::StatusCode::INTERNAL_SERVER_ERROR,
                ))
            }
        }
    }
}
