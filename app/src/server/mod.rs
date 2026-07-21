use admin::AdminApi;
use events::EventsApi;
use governor::Quota;
use opengraph::OpenGraph;
use pm::PMApi;
use poem::{
    EndpointExt, Route, Server,
    endpoint::StaticFilesEndpoint,
    get, handler,
    listener::TcpListener,
    middleware::{Cors, OpenTelemetryMetrics},
};
use poem_openapi::{OpenApi, OpenApiService, Tags, payload::Html};
use ratelimit::GovRateLimitMiddleware;
use std::num::NonZero;
use topic::TopicApi;
use tracing::info;
use user::UserApi;
use webhooks::WebhookApi;

use crate::{server::search::SearchApi, state::AppState};
// use tracing_mw::TraceId;

pub mod admin;
pub mod auth;
pub mod events;
pub mod mcp;
pub mod opengraph;
pub mod pm;
pub mod ratelimit;
pub mod search;
pub mod topic;
pub mod user;
pub mod webhooks;

#[derive(Tags)]
pub enum ApiTags {
    /// Topic Related Operations
    Topic,
    /// User Related Operations
    User,
    /// Events Related Operations
    Events,
    /// Search Related Operations
    Search,
    /// Admin Related Operations
    Admin,
    /// Webhooks Related Operations
    Webhooks,
}

fn get_api(_state: AppState) -> impl OpenApi {
    (
        TopicApi, UserApi, EventsApi, PMApi, SearchApi, AdminApi, WebhookApi,
    )
}

pub async fn start_http(state: AppState) {
    info!("Starting HTTP server");
    let api_service = OpenApiService::new(get_api(state.clone()), "Ethereum Forum", "0.0.1")
        .server("https://ethereum.forum/api")
        .server("http://localhost:3000/api")
        .description("Ethereum Forum API with JWT Bearer Token Authentication");

    let spec = api_service.spec_endpoint();

    let limiter = GovRateLimitMiddleware::new(
        Quota::per_minute(NonZero::new(120).unwrap()),
        Quota::per_minute(NonZero::new(60).unwrap()),
    );

    let opengraph = OpenGraph::new(&state);

    let api_service = api_service
        .with(limiter)
        // .with(TraceId::new(Arc::new(global::tracer("ethereum-forum"))))
        .with(OpenTelemetryMetrics::new());

    let path = std::path::Path::new("./www");

    let assets_endpoint = StaticFilesEndpoint::new(path.join("assets"));
    let spa_endpoint = StaticFilesEndpoint::new(path)
        .show_files_listing()
        .index_file("index.html")
        .fallback_to_index()
        .no_cache_index()
        .with(opengraph);

    let app = Route::new()
        .nest("/assets", assets_endpoint)
        .nest("/", spa_endpoint)
        .nest("/openapi.json", spec)
        .nest("/docs", get(get_openapi_docs))
        .nest("/api", api_service)
        .nest("/mcp", mcp::endpoint(state.clone()))
        .data(state)
        .with(Cors::new());

    Server::new(TcpListener::bind("0.0.0.0:3000"))
        .run(app)
        .await
        .unwrap();
}

#[handler]
async fn get_openapi_docs() -> Html<&'static str> {
    Html(include_str!("./index.html"))
}
