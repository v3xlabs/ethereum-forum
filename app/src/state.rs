use crate::{
    database::Database,
    modules::{
        discourse::{self, DiscourseService},
        ical::{self, ICalConfig},
        llm::LlmService,
        meili,
        pm::PMModule,
        sso::SSOService,
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
    pub pm: PMModule,
    pub sso: Option<SSOService>,
    pub llm: Option<LlmService>,
    pub cache: CacheService,
    pub meili: Option<meili::Client>,
}

impl AppStateInner {
    /// # Panics
    /// Panics if the environment variables for the database configuration are not set.
    pub async fn init() -> Self {
        // Load configuration from environment variables
        let database_config = Figment::new()
            .merge(Env::prefixed("DATABASE_"))
            .extract::<DatabaseConfig>()
            .expect("Failed to load database configuration");

        let database = Database::init(&database_config).await;

        let llm = LlmService::from_env();

        let cache = CacheService::default();

        let ical = ical::init_ical(Figment::new()).await;

        let discourse_configs = discourse::create_discourse_configs();
        let discourse = DiscourseService::new(discourse_configs);

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
            pm,
            llm,
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
