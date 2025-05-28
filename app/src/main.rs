use std::sync::Arc;

use anyhow::Error;
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

    let state = state::AppStateInner::init().await;
    let state = Arc::new(state);

    let discourse_state = state.clone();
    let discourse_handle = async_std::task::spawn(async move {
        discourse_state.clone().discourse.run(discourse_state).await;
    });
    let discourse2_state = state.clone();
    let discourse2_handle = async_std::task::spawn(async move {
        discourse2_state
            .clone()
            .discourse
            .fetch_periodically()
            .await;
    });
    let server_handle = async_std::task::spawn(server::start_http(state.clone()));

    join!(server_handle, discourse_handle, discourse2_handle);
    Ok(())
}
