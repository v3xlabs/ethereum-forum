use std::{sync::Arc, time::Duration};

use anyhow::Error;
use async_std::task::sleep;
use futures::join;

pub mod database;
pub mod models;
pub mod modules;
pub mod server;
pub mod state;
pub mod tmp;

#[async_std::main]
pub async fn main() -> Result<(), Error> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let state = state::AppStateInner::init().await;
    let state = Arc::new(state);

    let discourse_state = state.clone();
    let discourse_handle = async_std::task::spawn(async move {
        sleep(Duration::from_secs(5)).await;
        discourse_state
            .clone()
            .discourse
            .start_all_indexers(discourse_state)
            .await;
    });
    let github_state = state.clone();
    let github_handle = async_std::task::spawn(async move {
        sleep(Duration::from_secs(5)).await;
        github_state
            .clone()
            .github
            .start_all_indexers(github_state)
            .await;
    });

    let server_handle = async_std::task::spawn(server::start_http(state));

    join!(server_handle, discourse_handle, github_handle);
    Ok(())
}
