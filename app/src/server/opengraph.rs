use async_trait::async_trait;
use poem::web::Html;
use poem::IntoResponse;
use poem::{middleware::Middleware, Endpoint, Request, Response};
use regex::Regex;
use tracing::info;

use crate::models::topics::Topic;
use crate::state::AppState;

#[derive(Clone)]
pub struct OpenGraph {
    state: AppState,
}

impl OpenGraph {
    #[must_use]
    pub fn new(state: &AppState) -> Self {
        Self {
            state: state.clone(),
        }
    }
}

#[async_trait]
impl<E: Endpoint> Middleware<E> for OpenGraph {
    type Output = OpenGraphMiddlewareImpl<E>;

    fn transform(&self, ep: E) -> Self::Output {
        OpenGraphMiddlewareImpl {
            ep,
            state: self.state.clone(),
        }
    }
}

pub struct OpenGraphMiddlewareImpl<E> {
    ep: E,
    state: AppState,
}

impl<E: Endpoint> Endpoint for OpenGraphMiddlewareImpl<E> {
    type Output = Response;

    async fn call(&self, req: Request) -> poem::Result<Self::Output> {
        let route = req.uri().to_string();

        info!("OpenGraph request to route: {}", route);

        let mut opengraph_title: Option<String> = None;
        let mut opengraph_description: Option<String> = None;
        let mut opengraph_image: Option<String> = None;

        if route.starts_with("/t/") {
            let topic_id = route.split('/').nth(2).unwrap_or_default();
            let topic_id = topic_id.parse::<i32>().ok();
            info!("Topic ID: {:?}", topic_id);
            if let Some(topic_id) = topic_id {
                let topic = Topic::get_by_topic_id(topic_id, &self.state).await;

                if let Ok(topic) = topic {
                    let first_post = topic.get_first_post(&self.state).await.ok();

                    //
                    info!("OpenGraph request to topic: {}", topic.title);
                    opengraph_title = Some(topic.title);
                    opengraph_description = first_post.and_then(|post| post.cooked).map(|cooked| {
                        let regex = Regex::new(r"<[^>]*?>").unwrap();
                        regex.replace_all(&cooked, "").to_string()
                    });
                    opengraph_image = topic.image_url;
                }
            }
        }

        // Process the request normally.
        let x = self.ep.call(req).await?;
        let mut response = x.into_response();

        if opengraph_title.is_some() || opengraph_description.is_some() || opengraph_image.is_some()
        {
            // modify the html in the body of the response such that it has opengraph head tags
            let body = response.take_body();
            let body = body.into_bytes().await.unwrap();
            let mut body = String::from_utf8(body.to_vec()).unwrap();

            if let Some(title) = opengraph_title {
                body = Regex::new(r#"property="og:title" content="[^"]*?""#)
                    .unwrap()
                    .replace(&body, format!("property=\"og:title\" content=\"{title}\""))
                    .to_string();
                body = Regex::new(r#"name="twitter:title" content="[^"]*?""#)
                    .unwrap()
                    .replace(&body, format!("name=\"twitter:title\" content=\"{title}\""))
                    .to_string();
                body = Regex::new(r"<title>[^<]*?</title>")
                    .unwrap()
                    .replace(&body, format!("<title>{title}</title>"))
                    .to_string();
            }

            if let Some(description) = opengraph_description {
                body = Regex::new(r#"property="og:description" content="[^"]*?""#)
                    .unwrap()
                    .replace(
                        &body,
                        format!("property=\"og:description\" content=\"{description}\""),
                    )
                    .to_string();
                body = Regex::new(r#"name="twitter:description" content="[^"]*?""#)
                    .unwrap()
                    .replace(
                        &body,
                        format!("name=\"twitter:description\" content=\"{description}\""),
                    )
                    .to_string();
            }

            if let Some(image) = opengraph_image {
                body = Regex::new(r#"property="og:image" content="[^"]*?""#)
                    .unwrap()
                    .replace(&body, format!("property=\"og:image\" content=\"{image}\""))
                    .to_string();
                body = Regex::new(r#"name="twitter:image" content="[^"]*?""#)
                    .unwrap()
                    .replace(&body, format!("name=\"twitter:image\" content=\"{image}\""))
                    .to_string();
            }

            response = Html(body).into_response();
        }

        Ok(response)
    }
}
