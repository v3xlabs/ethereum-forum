use poem::web::Data;
use poem::Result;
use poem_openapi::param::{Path, Query};
use poem_openapi::payload::Json;
use poem_openapi::{Object, OpenApi};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use crate::models::discourse::user::{DiscourseUserProfile, DiscourseUserSummaryResponse};
use crate::modules::discourse::LResult;
use crate::modules::sso::{AuthResponse, UserInfo};
use crate::state::AppState;
use crate::server::ApiTags;
use crate::server::auth::AuthUser;

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct UserApi;

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct TokenValidationResponse {
    pub valid: bool,
    pub user: Option<UserInfo>,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct SSOProvidersResponse {
    pub providers: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct LoginResponse {
    pub redirect_url: String,
}

#[derive(Debug, Serialize, Deserialize, Object)]
pub struct UserProfileResponse {
    pub user_id: String,
    pub email: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub created_at: String,
    pub provider: String,
    pub expires_at: i64,
    pub token_expiring_soon: bool,
}

#[OpenApi]
impl UserApi {
    /// /users
    ///
    /// List users
    #[oai(path = "/users", method = "get", tag = "ApiTags::User")]
    async fn list(
        &self,
        _state: Data<&AppState>,
    ) -> Result<Json<serde_json::Value>> {
        Ok(Json(serde_json::Value::Null))
    }

    /// /user/profile
    ///
    /// Get authenticated user's profile
    #[oai(path = "/user/profile", method = "get", tag = "ApiTags::User")]
    async fn get_profile(
        &self,
        _state: Data<&AppState>,
        auth_user: AuthUser,
    ) -> Result<Json<UserProfileResponse>> {
        let user = &auth_user.0.user;
        let claims = &auth_user.0.claims;

        Ok(Json(UserProfileResponse {
            user_id: user.user_id.to_string(),
            email: user.email.clone().unwrap_or_default(),
            display_name: user.display_name.clone(),
            avatar_url: user.avatar_url.clone(),
            created_at: user.created_at.to_rfc3339(),
            provider: claims.provider.clone(),
            expires_at: auth_user.0.expires_at(),
            token_expiring_soon: auth_user.0.is_token_expiring_soon(),
        }))
    }

    /// /du/:discourse_id/:username
    ///
    /// Get user profile
    #[oai(path = "/du/:discourse_id/:username", method = "get", tag = "ApiTags::User")]
    async fn get_user(
        &self,
        state: Data<&AppState>,
        #[oai(style = "simple")] discourse_id: Path<String>,
        #[oai(style = "simple")] username: Path<String>,
    ) -> Result<Json<DiscourseUserProfile>> {
        let user = match state.discourse.fetch_discourse_user_cached(&discourse_id, &username).await {
            Ok(LResult::Success(user)) => user,
            Ok(LResult::Failed(error)) => {
                tracing::error!("Error fetching user: {}", error);
                return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
            }
            Err(e) => {
                tracing::error!("Error fetching user: {:?}", e);
                return Err(poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR));
            }
        };

        Ok(Json(user))
    }

    /// /du/:discourse_id/:username/summary
    ///
    /// Get user summary
    #[oai(path = "/du/:discourse_id/:username/summary", method = "get", tag = "ApiTags::User")]
    async fn get_user_summary(
        &self,
        state: Data<&AppState>,
        #[oai(style = "simple")] discourse_id: Path<String>,
        #[oai(style = "simple")] username: Path<String>,
    ) -> Result<Json<DiscourseUserSummaryResponse>> {
        let summary = match state.discourse.fetch_discourse_user_summary_cached(&discourse_id, &username).await {
            Ok(LResult::Success(summary)) => summary,
            Ok(LResult::Failed(error)) => {
                tracing::error!("Error fetching user summary: {}", error);
                return Err(poem::Error::from_status(StatusCode::NOT_FOUND));
            }
            Err(e) => {
                tracing::error!("Error fetching user summary: {:?}", e);
                return Err(poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR));
            }
        };

        Ok(Json(summary))
    }

    /// /user/sso/providers
    /// 
    /// Get available SSO providers
    #[oai(path = "/user/sso/providers", method = "get", tag = "ApiTags::User")]
    async fn get_sso_providers(
        &self,
        state: Data<&AppState>,
    ) -> Result<Json<SSOProvidersResponse>> {
        let providers = if let Some(sso) = &state.sso {
            sso.config.providers.keys().cloned().collect()
        } else {
            vec![]
        };

        Ok(Json(SSOProvidersResponse { providers }))
    }

    /// /user/sso/:sso_id/login
    /// 
    /// Login with SSO - returns redirect URL
    #[oai(path = "/user/sso/:sso_id/login", method = "get", tag = "ApiTags::User")]
    async fn login(
        &self,
        state: Data<&AppState>,
        #[oai(style = "simple")] sso_id: Path<String>,
    ) -> Result<Json<LoginResponse>> {
        let sso_service = state.sso.as_ref()
            .ok_or_else(|| {
                tracing::error!("SSO service not configured");
                poem::Error::from_status(StatusCode::SERVICE_UNAVAILABLE)
            })?;

        let (auth_url, session) = sso_service.get_authorization_url(&sso_id)
            .map_err(|e| {
                tracing::error!("Error getting authorization URL: {:?}", e);
                poem::Error::from_status(StatusCode::BAD_REQUEST)
            })?;

        // In a real implementation, you'd store the session somewhere (Redis, database, etc.)
        tracing::info!("Generated SSO session for provider {}: {:?}", &*sso_id, session);

        Ok(Json(LoginResponse {
            redirect_url: auth_url
        }))
    }

    /// /user/sso/:sso_id/callback
    /// 
    /// SSO callback endpoint - exchanges code for JWT token
    #[oai(path = "/user/sso/:sso_id/callback", method = "get", tag = "ApiTags::User")]
    async fn sso_callback(
        &self,
        state: Data<&AppState>,
        #[oai(style = "simple")] sso_id: Path<String>,
        #[oai(style = "simple")] code: Query<String>,
        #[oai(style = "simple")] _state_param: Query<Option<String>>,
    ) -> Result<Json<AuthResponse>> {
        let sso_service = state.sso.as_ref()
            .ok_or_else(|| {
                tracing::error!("SSO service not configured");
                poem::Error::from_status(StatusCode::SERVICE_UNAVAILABLE)
            })?;

        // In a real implementation, you'd retrieve the session from storage
        let session = crate::modules::sso::AuthSession {
            csrf_token: "dummy".to_string(),
            nonce: "dummy".to_string(),
            provider_id: sso_id.clone(),
            redirect_uri: "http://localhost:3000/auth/callback".to_string(),
        };

        let (jwt_token, user) = sso_service.exchange_code_for_token(&sso_id, &code, &session, &state.database.pool).await
            .map_err(|e| {
                tracing::error!("Error exchanging code for token: {:?}", e);
                poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
            })?;

        // Extract user info from the token to return in response
        let claims = sso_service.validate_jwt_token(&jwt_token)
            .map_err(|e| {
                tracing::error!("Error validating generated token: {:?}", e);
                poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
            })?;

        Ok(Json(AuthResponse {
            token: jwt_token,
            user,
            expires_at: claims.exp,
        }))
    }

    /// /user/token/validate
    /// 
    /// Validate JWT token
    #[oai(path = "/user/token/validate", method = "post", tag = "ApiTags::User")]
    async fn validate_token(
        &self,
        state: Data<&AppState>,
        token: Query<String>,
    ) -> Result<Json<TokenValidationResponse>> {
        let sso_service = state.sso.as_ref()
            .ok_or_else(|| {
                tracing::error!("SSO service not configured");
                poem::Error::from_status(StatusCode::SERVICE_UNAVAILABLE)
            })?;

        match sso_service.validate_jwt_token(&token) {
            Ok(claims) => {
                let user_info = UserInfo {
                    sub: claims.sub,
                    email: claims.email,
                    name: claims.name,
                };

                Ok(Json(TokenValidationResponse {
                    valid: true,
                    user: Some(user_info),
                    expires_at: Some(claims.exp),
                }))
            }
            Err(e) => {
                tracing::debug!("Token validation failed: {:?}", e);
                Ok(Json(TokenValidationResponse {
                    valid: false,
                    user: None,
                    expires_at: None,
                }))
            }
        }
    }

    /// /user/:user_id
    /// Get a Forum user by its user_id
    #[oai(path = "/user/:user_id", method = "get", tag = "ApiTags::User")]
    async fn get_user_by_id(
        &self,
        state: Data<&AppState>,
        #[oai(style = "simple")] user_id: Path<String>,
    ) -> Result<poem_openapi::payload::PlainText<String>> {
        let user_id = match uuid::Uuid::parse_str(&user_id) {
            Ok(id) => id,
            Err(_) => {
                tracing::error!("Invalid user_id format");
                return Err(poem::Error::from_status(StatusCode::BAD_REQUEST));
            }
        };

        let user = sqlx::query!("SELECT * FROM users WHERE user_id = $1", user_id)
            .fetch_one(&state.database.pool)
            .await
            .map_err(|e| {
                tracing::error!("Error fetching user by ID: {:?}", e);
                poem::Error::from_status(StatusCode::NOT_FOUND)
            })?;

        Ok(poem_openapi::payload::PlainText(
            user.display_name
                .unwrap_or_else(|| "Unknown User".to_string()),
        ))
    }
}
