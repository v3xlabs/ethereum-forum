use crate::{
    database::Database,
    modules::{
        discourse::DiscourseService,
        ical::{self, ICalConfig},
        pm::PMModule,
        sso::SSOService,
        workshop::WorkshopService,
    },
    tmp::CacheService,
};
use figment::{Figment, providers::Env};
use meilisearch_sdk::client::Client;
use serde::{Deserialize, Serialize};
use std::{env, sync::Arc};

pub type AppState = Arc<AppStateInner>;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
}

pub struct AppStateInner {
    pub database: Database,
    pub ical: Option<ICalConfig>,
    pub discourse: DiscourseService,
    pub pm: PMModule,
    pub sso: Option<SSOService>,
    pub workshop: WorkshopService,
    pub cache: CacheService,
    pub meili: Option<Client>,
}

impl AppStateInner {
    /// # Panics
    /// Panics if the environment variables for the database configuration are not set.
    /// Panics if the OpenAI-compatible API key or base URL for the intelligence is not set.
    pub async fn init() -> Self {
        // Load configuration from environment variables
        let database_config = Figment::new()
            .merge(Env::prefixed("DATABASE_"))
            .extract::<DatabaseConfig>()
            .expect("Failed to load database configuration");

        let database = Database::init(&database_config).await;

        let workshop = WorkshopService::init().await;

        let cache = CacheService::default();

        let ical = ical::init_ical(Figment::new()).await;

        let discourse = DiscourseService::default();

        let pm = PMModule::default();

        let meili = match (env::var("MEILI_HOST"), env::var("MEILI_KEY")) {
            (Ok(meili_url), Ok(meili_key)) => {
                let client = Client::new(&meili_url, Some(meili_key.as_str()))
                    .expect("Failed to create MeiliSearch client");
                match client.get_version().await {
                    Ok(version) => {
                        tracing::info!("Connected to MeiliSearch: version {}", version.commit_sha);
                        Some(client)
                    }
                    Err(e) => {
                        tracing::error!("Failed to connect to MeiliSearch: {}", e);
                        None
                    }
                }
            }
            _ => None,
        };

        let sso = match SSOService::new(Figment::new().merge(Env::raw())).await {
            Ok(service) => {
                tracing::info!("SSO service initialized successfully");
                Some(service)
            }
            Err(e) => {
                tracing::info!(
                    "SSO service initialization failed: {}. SSO will be disabled.",
                    e
                );
                None
            }
        };

        Self {
            database,
            ical,
            cache,
            discourse,
            pm,
            workshop,
            sso,
            meili,
        }
    }
}

impl std::fmt::Debug for AppStateInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppStateInner")
            // .field("database", &self.database)
            // .field("cache", &self.cache)
            .finish()
    }
}
