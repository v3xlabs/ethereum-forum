use crate::{
    database::Database,
    modules::{
        discourse::DiscourseService,
        ical::{self, ICalConfig},
        pm::PMModule,
    },
    tmp::CacheService,
};
use figment::{providers::Env, Figment};
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

    //
    pub cache: CacheService,
}

impl AppStateInner {
    pub async fn init() -> Self {
        // Load configuration from environment variables
        let database_config = Figment::new()
            .merge(Env::prefixed("DATABASE_"))
            .extract::<DatabaseConfig>()
            .expect("Failed to load database configuration");

        let database = Database::init(&database_config).await;

        let cache = CacheService::default();

        let ical = ical::init_ical(Figment::new());

        let discourse = DiscourseService::default();

        let pm = PMModule;

        Self {
            database,
            ical,
            discourse,
            pm,
            cache,
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
