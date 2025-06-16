use async_trait::async_trait;
use poem::IntoResponse;
use poem::web::{Data, Html};
use poem::{Endpoint, Request, Response, middleware::Middleware};
use poem_openapi::{ApiResponse, Object, OpenApi, param::Path, payload::Binary};
use regex::Regex;
use resvg::render;
use serde::Serialize;
use std::ops::Deref;
use tiny_skia::{Pixmap, Transform};
use tracing::info;
use usvg::{Options, Tree};
use usvg_remote_resolvers::HrefStringResolver;
use usvg_remote_resolvers::reqwest::ReqwestResolver;

use crate::models::topics::Topic;
use crate::server::ApiTags;
use crate::state::AppState;

#[derive(ApiResponse)]
enum WebPImageResponse {
    /// WebP image
    #[oai(status = 200, content_type = "image/webp")]
    Ok(Binary<Vec<u8>>),
}

fn format_count(count: i32) -> String {
    if count >= 1000 {
        format!("{}k", count / 1000)
    } else {
        count.to_string()
    }
}

#[derive(Clone)]
pub struct OpenGraph {
    state: AppState,
}

impl OpenGraph {
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

impl<E: Endpoint> Endpoint for OpenGraphMiddlewareImpl<E>
where
    E: Endpoint,
{
    type Output = Response;

    async fn call(&self, req: Request) -> poem::Result<Self::Output> {
        let route = req.uri().to_string();

        info!("OpenGraph request to route: {}", route);

        let mut opengraph_title: Option<String> = None;
        let mut opengraph_description: Option<String> = None;
        let mut opengraph_image: Option<String> = None;

        if route.starts_with("/t/") {
            let split = route.split("/").collect::<Vec<&str>>();
            // parse path parameters
            let discourse_id = split.get(2).unwrap_or(&"magicians").to_string();
            let topic_id = split.get(3).unwrap_or(&"").to_string();
            let topic_id = topic_id.parse::<i32>().ok();
            info!("Topic ID: {:?}", topic_id);
            if let Some(topic_id) = topic_id {
                let topic = Topic::get_by_topic_id(&discourse_id, topic_id, &self.state).await;

                if let Ok(topic) = topic {
                    let first_post = topic.get_first_post(&self.state).await.ok();

                    //
                    info!("OpenGraph request to topic: {}", topic.title);
                    opengraph_title = Some(topic.title);
                    opengraph_description = first_post.and_then(|post| post.cooked).map(|cooked| {
                        let regex = Regex::new(r#"<[^>]*?>"#).unwrap();
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
                    .replace(
                        &body,
                        format!("property=\"og:title\" content=\"{}\"", title),
                    )
                    .to_string();
                body = Regex::new(r#"name="twitter:title" content="[^"]*?""#)
                    .unwrap()
                    .replace(
                        &body,
                        format!("name=\"twitter:title\" content=\"{}\"", title),
                    )
                    .to_string();
                body = Regex::new(r#"<title>[^<]*?</title>"#)
                    .unwrap()
                    .replace(&body, format!("<title>{}</title>", title))
                    .to_string();
            }

            if let Some(description) = opengraph_description {
                body = Regex::new(r#"property="og:description" content="[^"]*?""#)
                    .unwrap()
                    .replace(
                        &body,
                        format!("property=\"og:description\" content=\"{}\"", description),
                    )
                    .to_string();
                body = Regex::new(r#"name="twitter:description" content="[^"]*?""#)
                    .unwrap()
                    .replace(
                        &body,
                        format!("name=\"twitter:description\" content=\"{}\"", description),
                    )
                    .to_string();
            }

            if let Some(image) = opengraph_image {
                body = Regex::new(r#"property="og:image" content="[^"]*?""#)
                    .unwrap()
                    .replace(
                        &body,
                        format!("property=\"og:image\" content=\"{}\"", image),
                    )
                    .to_string();
                body = Regex::new(r#"name="twitter:image" content="[^"]*?""#)
                    .unwrap()
                    .replace(
                        &body,
                        format!("name=\"twitter:image\" content=\"{}\"", image),
                    )
                    .to_string();
            }

            response = Html(body).into_response();
        }

        Ok(response)
    }
}

const WIDTH: u32 = 1200;
const HEIGHT: u32 = 630;

#[derive(Debug, Serialize, Object)]
pub struct OpenGraphApi;

#[OpenApi]
impl OpenGraphApi {
    /// This route returns the OpenGraph image for a topic.
    #[oai(
        path = "/og/t/:discourse_id/:topic_id",
        method = "get",
        tag = "ApiTags::OpenGraph",
        hidden
    )]
    async fn get_opengraph(
        &self,
        state: Data<&crate::state::AppState>,
        #[oai(style = "simple")] discourse_id: Path<String>,
        #[oai(style = "simple")] topic_id: Path<i32>,
    ) -> WebPImageResponse {
        let remote_resource_resolver = ReqwestResolver::default();

        let template = liquid::ParserBuilder::with_stdlib()
            .build()
            .unwrap()
            .parse(include_str!(
                "templates/opengraph/t/:discourse_id/:topic_id.svg"
            ))
            .unwrap();

        let topic = Topic::get_by_topic_id(&discourse_id.0, topic_id.0, &state)
            .await
            .unwrap_or_else(|_| {
                panic!(
                    "Failed to load topic with discourse_id: {} and topic_id: {}",
                    discourse_id.0, topic_id.0
                );
            });

        let base_url = match discourse_id.0.as_str() {
            "magicians" => "https://ethereum-magicians.org",
            "research" => "https://ethresear.ch",
            _ => "https://ethereum-magicians.org", // This case should not happen, but we handle it gracefully
        };

        let avatars: Vec<String> = topic
            .extra
            .as_ref()
            .and_then(|extra| extra.get("details"))
            .and_then(|details| details.get("participants"))
            .and_then(|participants| participants.as_array())
            .map(|participants| {
                participants
                    .iter()
                    .filter_map(|p| {
                        p.get("avatar_template").and_then(|a| a.as_str()).map(|s| {
                            let avatar_url = s.replace("{size}", "64");
                            format!("{}{}", base_url, avatar_url)
                        })
                    })
                    .collect()
            })
            .unwrap_or_else(Vec::new);

        let globals = liquid::object!({
            "topictitle": topic.title,
            "views": format_count(topic.view_count),
            "likes": format_count(topic.like_count),
            "timeago": "5 days ago",
            "avatars": avatars,
        });

        let svg = template.render(&globals).unwrap();

        let mut options = Options::default();

        options.image_href_resolver.resolve_string = remote_resource_resolver.into_fn();

        let mut fontdb = usvg::fontdb::Database::new();
        fontdb.load_font_data(
            include_bytes!("templates/RobotoCondensed-VariableFont_wght.ttf").to_vec(),
        );
        options.fontdb = std::sync::Arc::new(fontdb);

        let tree = Tree::from_str(&svg, &options)
            .unwrap_or_else(|_| Tree::from_str("<svg></svg>", &options).unwrap());

        let mut pixmap = Pixmap::new(1200, 630).unwrap_or_else(|| Pixmap::new(1, 1).unwrap());

        render(&tree, Transform::default(), &mut pixmap.as_mut());

        let encoded_buffer =
            webp::Encoder::new(pixmap.data(), webp::PixelLayout::Rgba, WIDTH, HEIGHT)
                .encode_lossless();
        let result = encoded_buffer.deref();

        WebPImageResponse::Ok(Binary(result.to_vec()))
    }
}
