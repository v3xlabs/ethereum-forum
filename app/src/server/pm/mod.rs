use poem::{web::Data, Result};
use poem_openapi::param::Path;
use poem_openapi::{payload::Json, Object, OpenApi};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::models::pm::PMMeetingData;
use crate::server::ApiTags;
use crate::state::AppState;

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct PMApi;

#[OpenApi]
impl PMApi {
    /// /pm
    ///
    /// Get PM data
    #[oai(path = "/pm/:issue_id", method = "get", tag = "ApiTags::Events")]
    async fn get_by_id(
        &self,
        state: Data<&AppState>,
        #[oai(style = "simple")] issue_id: Path<u32>,
    ) -> Result<Json<PMMeetingData>> {
        let pm = state
            .pm
            .get_by_issue_id(issue_id.0)
            .await
            .or(Err(poem::Error::from_status(StatusCode::NOT_FOUND)))?;

        info!("PM data: {:?}", pm);
        Ok(Json(pm))
    }
}
