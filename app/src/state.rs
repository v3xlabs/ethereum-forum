use crate::{
    database::Database,
    modules::{
        blog::BlogService,
        discourse::{self, DiscourseService},
        ical::{self, ICalConfig},
        meili,
        pm::PMModule,
        sso::SSOService,
        workshop::WorkshopService,
    },
    tmp::CacheService,
};
use figment::{Figment, providers::Env};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub type AppState = Arc<AppStateInner>;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
}

pub struct AppStateInner {
    pub database: Database,
    pub ical: Option<ICalConfig>,
    pub discourse: DiscourseService,
    pub blog: BlogService,
    pub pm: PMModule,
    pub sso: Option<SSOService>,
    pub workshop: WorkshopService,
    pub cache: CacheService,
    pub meili: Option<meili::Client>,
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

        let discourse_configs = discourse::create_discourse_configs();
        let discourse = DiscourseService::new(discourse_configs);

        let blog = BlogService::new();

        let pm = PMModule::default();

        let meili = meili::init_meili().await;

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
            blog,
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
