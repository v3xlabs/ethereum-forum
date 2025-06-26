pub use meilisearch_sdk::client::Client;

pub async fn init_meili() -> Option<Client> {
    match (std::env::var("MEILI_HOST"), std::env::var("MEILI_KEY")) {
        (Ok(meili_url), Ok(meili_key)) => {
            let client = Client::new(&meili_url, Some(meili_key.as_str()))
                .expect("Failed to create MeiliSearch client");
            match client.get_version().await {
                Ok(version) => {
                    tracing::info!("Connected to MeiliSearch: version {}", version.commit_sha);
                    
                    // Configure the forum index
                    if let Err(e) = configure_forum_index(&client).await {
                        tracing::error!("Failed to configure forum index: {}", e);
                        return None;
                    }
                    
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

async fn configure_forum_index(client: &Client) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut forum_index = client.index("forum");
    
    // Set filterable attributes for the forum index
    let filterable_attributes = vec![
        "entity_type".to_string(),
        "topic_id".to_string(), 
        "user_id".to_string(),
        "username".to_string(),
        "pm_issue".to_string(),
        "post_id".to_string(),
        "discourse_id".to_string(),
    ];
    
    // Set searchable attributes for better search experience
    let searchable_attributes = vec![
        "title".to_string(),
        "cooked".to_string(),
        "slug".to_string(),
        "username".to_string(),
    ];
    
    // Configure the index settings
    match forum_index.set_filterable_attributes(&filterable_attributes).await {
        Ok(_) => {
            tracing::info!("Successfully configured filterable attributes for forum index");
        }
        Err(e) => {
            tracing::error!("Failed to set filterable attributes: {}", e);
            return Err(Box::new(e));
        }
    }
    
    match forum_index.set_searchable_attributes(&searchable_attributes).await {
        Ok(_) => {
            tracing::info!("Successfully configured searchable attributes for forum index");
        }
        Err(e) => {
            tracing::error!("Failed to set searchable attributes: {}", e);
            return Err(Box::new(e));
        }
    }
    
    // Set the primary key for the index
    match forum_index.set_primary_key("entity_id").await {
        Ok(_) => {
            tracing::info!("Successfully configured primary key for forum index");
        }
        Err(e) => {
            tracing::error!("Failed to set primary key: {}", e);
            return Err(Box::new(e));
        }
    }
    
    Ok(())
}
