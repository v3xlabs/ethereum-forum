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
use openai::{
    Credentials,
    chat::{ChatCompletionMessage, ChatCompletionMessageRole},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub type AppState = Arc<AppStateInner>;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
}

pub struct WorkshopPrompts {
    pub summerize: ChatCompletionMessage,
}
pub struct WorkshopState {
    pub crendentials: Credentials,
    pub prompts: WorkshopPrompts,
}

pub struct AppStateInner {
    pub database: Database,
    pub ical: Option<ICalConfig>,
    pub discourse: DiscourseService,
    pub pm: PMModule,
    pub workshop: WorkshopState,
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

        let workshop = WorkshopState {
            crendentials: Credentials::from_env(),
            prompts: WorkshopPrompts {
                summerize: ChatCompletionMessage {
                    role: ChatCompletionMessageRole::System,
                    content: Some("You are an expert ethereum magician and are tasked with summarizing threads on the ethereum magicians forum. You will be provided with a thread and you will summarize the thread in a way that is easy to understand for a layman. You will also provide an overview of the stances of the top conversers in the thread, sorted by their stance (pro, against, other). You will also provide a summary of each person's stance.
When evaluating a heated argument ensure to capture all sides and create an overview at the end of our output showcasing the stances of the top conversers. So those who are pro, those who are against, and potentially those with alternative solutions.
Ensure to output the list sorted by overal stance (pro, against, other), and then by person, with a small summary of that persons stance.
Return valid markdown, without any images. You may use simple code snippets.".to_string()),
                    name: None,
                    function_call: None,
                    tool_call_id: None,
                    tool_calls: None,
                },
            },
        };

        let cache = CacheService::default();

        let ical = ical::init_ical(Figment::new()).await;

        let discourse = DiscourseService::default();

        let pm = PMModule::default();

        Self {
            database,
            ical,
            cache,
            discourse,
            pm,
            workshop,
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
