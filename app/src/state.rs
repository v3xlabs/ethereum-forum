use crate::{
    database::Database,
    modules::{
        discourse::DiscourseService,
        ical::{self, ICalConfig},
        pm::PMModule,
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
    pub meili: Option<Client>,
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

        let ical = ical::init_ical(Figment::new()).await;

        let discourse = DiscourseService::default();

        let pm = PMModule::default();

        let meili = match (env::var("MEILI_HOST"), env::var("MEILI_KEY")) {
            (Ok(meili_url), Ok(meili_key)) => {
                let client = Client::new(&meili_url, Some(meili_key.as_str()))
                    .expect("Failed to create MeiliSearch client");
                let _ = client
                    .index("posts")
                    .set_separator_tokens(&vec!["<".to_string(), ">".to_string()])
                    .await;
                Some(client)
            }
            _ => None,
        };

        Self {
            database,
            ical,
            cache,
            discourse,
            pm,
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
