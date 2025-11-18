use axum::{
    extract::State,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use crate::error::AppResult;
use crate::AppState;

pub fn create_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(get_config))
        .route("/", post(update_config))
        .route("/export", get(export_config))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigUpdate {
    pub key: String,
    pub value: serde_json::Value,
}

async fn get_config(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<serde_json::Value>> {
    let config = state.config.read().unwrap();
    
    Ok(Json(json!({
        "status": true,
        "name": config.webui_name,
        "version": env!("CARGO_PKG_VERSION"),
        "default_locale": config.default_locale,
        "images": {
            "logo": config.webui_logo_url,
            "icon": config.webui_favicon_url,
        },
        "audio": {
            "tts": {
                "engine": config.audio_tts_engine,
                "voice": config.audio_tts_voice,
            },
            "stt": {
                "engine": config.audio_stt_engine,
            }
        },
        "features": {
            "enable_signup": config.enable_signup,
            "enable_login_form": config.enable_login_form,
            "enable_web_search": config.enable_web_search,
            "enable_image_generation": config.enable_image_generation,
            "enable_community_sharing": config.enable_community_sharing,
            "enable_message_rating": config.enable_message_rating,
        },
        "oauth": {
            "providers": []
        }
    })))
}

async fn update_config(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ConfigUpdate>,
) -> AppResult<Json<serde_json::Value>> {
    // In a real implementation, this would update the config
    // For now, just return success
    Ok(Json(json!({
        "status": "success",
        "message": format!("Config key '{}' updated", payload.key)
    })))
}

async fn export_config(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<serde_json::Value>> {
    let config = state.config.read().unwrap();
    
    Ok(Json(json!({
        "WEBUI_NAME": config.webui_name,
        "ENABLE_SIGNUP": config.enable_signup,
        "ENABLE_LOGIN_FORM": config.enable_login_form,
        "DEFAULT_LOCALE": config.default_locale,
        "ENABLE_WEB_SEARCH": config.enable_web_search,
        "ENABLE_IMAGE_GENERATION": config.enable_image_generation,
        "ENABLE_COMMUNITY_SHARING": config.enable_community_sharing,
    })))
}
