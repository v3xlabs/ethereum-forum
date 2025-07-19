use discourse_webhooks::{
    PostWebhookEvent, TopicWebhookEvent, WebhookError, WebhookEventHandler, WebhookProcessor,
};
use poem::{Result, web::Data};
use poem_openapi::param::Header;
use poem_openapi::{Object, OpenApi, payload::Json};
use serde::{Deserialize, Serialize};
use tracing::info;

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
}

impl DiscourseEventHandler {
    fn new(instance: String) -> Self {
        Self { instance }
    }
}

impl WebhookEventHandler for DiscourseEventHandler {
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn handle_ping(&mut self) -> std::result::Result<(), Self::Error> {
        info!("Received ping event from instance: {}", self.instance);
        Ok(())
    }

    fn handle_topic_created(
        &mut self,
        event: &TopicWebhookEvent,
    ) -> std::result::Result<(), Self::Error> {
        info!(
            "Topic created on instance {}: {}",
            self.instance, event.topic.title
        );
        Ok(())
    }

    fn handle_post_created(
        &mut self,
        event: &PostWebhookEvent,
    ) -> std::result::Result<(), Self::Error> {
        info!(
            "Post created on instance {}: Topic {}",
            self.instance, event.post.topic_id
        );
        Ok(())
    }

    // Add other handler methods as needed...
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
        _state: Data<&AppState>,
        body: Json<serde_json::Value>,
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

        let mut handler = DiscourseEventHandler::new(instance.to_string());

        match processor.process_json(
            &mut handler,
            discourse_event.as_str(),
            body.0,
            Some(signature.0.as_str()),
        ) {
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
