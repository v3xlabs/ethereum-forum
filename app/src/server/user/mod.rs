use poem::web::Data;
use poem::Result;
use poem_openapi::payload::Json;
use poem_openapi::{Object, OpenApi};
use serde::{Deserialize, Serialize};

use crate::server::ApiTags;
use crate::state::AppState;

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct UserApi;

#[OpenApi]
impl UserApi {
    /// /users
    ///
    /// List users
    #[oai(path = "/users", method = "get", tag = "ApiTags::User")]
    async fn list(&self, _state: Data<&AppState>) -> Result<Json<serde_json::Value>> {
        // let party = Party::create(&user.user_id, state.0).await.map_err(|e| {
        //     tracing::error!("Error creating party: {:?}", e);
        //     poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
        // })?;

        // Ok(Json(PartyCreateResponse {
        //     id: party.party_id,
        //     created_at: party.created_at.to_rfc3339(),
        // }))
        Ok(Json(serde_json::Value::Null))
    }
}
