pub use meilisearch_sdk::client::Client;

pub async fn init_meili() -> Option<Client> {
    match (std::env::var("MEILI_HOST"), std::env::var("MEILI_KEY")) {
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
    }
}
