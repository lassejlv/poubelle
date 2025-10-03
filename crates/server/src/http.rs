use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use engine::{Engine, QueryResult};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;

#[derive(Deserialize)]
struct QueryRequest {
    query: String,
}

#[derive(Serialize)]
#[serde(untagged)]
enum QueryResponse {
    Rows {
        rows: Vec<HashMap<String, JsonValue>>,
    },
    Success {
        message: String,
    },
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

enum ApiError {
    Engine(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::Engine(msg) => (StatusCode::BAD_REQUEST, msg),
        };

        (status, Json(ErrorResponse { error: message })).into_response()
    }
}

async fn query_handler(
    State(engine): State<Arc<Mutex<Engine>>>,
    Json(payload): Json<QueryRequest>,
) -> Result<Json<QueryResponse>, ApiError> {
    let mut engine = engine.lock().await;

    let result = engine
        .execute_query(&payload.query)
        .map_err(|e| ApiError::Engine(format!("{}", e)))?;

    let response = match result {
        QueryResult::Success(msg) => QueryResponse::Success { message: msg },
        QueryResult::Rows(rows) => {
            let parsed_rows: Vec<HashMap<String, JsonValue>> = rows
                .into_iter()
                .map(|row| {
                    row.data
                        .into_iter()
                        .map(|(k, v)| {
                            let json_value = match v {
                                storage::Value::Int(i) => JsonValue::Number(i.into()),
                                storage::Value::Text(s) => JsonValue::String(s),
                                storage::Value::Null => JsonValue::Null,
                            };
                            (k, json_value)
                        })
                        .collect()
                })
                .collect();

            QueryResponse::Rows { rows: parsed_rows }
        }
    };

    Ok(Json(response))
}

async fn health_handler() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "poubelle"
    }))
}

pub fn create_router(engine: Arc<Mutex<Engine>>) -> Router {
    Router::new()
        .route("/query", post(query_handler))
        .route("/health", axum::routing::get(health_handler))
        .layer(CorsLayer::permissive())
        .with_state(engine)
}

pub async fn start_http_server(
    engine: Arc<Mutex<Engine>>,
    host: String,
    port: String,
) -> anyhow::Result<()> {
    let app = create_router(engine);
    let bind_addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;

    println!("HTTP API listening on {}", bind_addr);
    axum::serve(listener, app).await?;

    Ok(())
}
