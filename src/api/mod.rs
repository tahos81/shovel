use crate::common::types::CairoUint256;
use crate::db;
use crate::file_storage::metadata::store_metadata;
use crate::file_storage::*;
use crate::rpc;
use crate::rpc::metadata::token;
use axum::routing::post;
use axum::routing::Router;
use axum::Json;
use color_eyre::eyre::Report;
use reqwest::StatusCode;
use serde::Deserialize;
use serde::Serialize;
use starknet::core::types::FieldElement;
use starknet::providers::jsonrpc::models::BlockId;
use starknet::providers::jsonrpc::models::BlockTag::Latest;

#[derive(Debug, Serialize, Deserialize)]
struct Payload {
    contract_address: FieldElement,
    token_id_low: FieldElement,
    token_id_high: FieldElement,
}

pub fn get_app() -> Router<()> {
    Router::new().route("/refresh", post(handler))
}

async fn handler(Json(payload): Json<Payload>) -> Result<StatusCode, (StatusCode, String)> {
    let rpc = rpc::connect().map_err(internal_error)?;
    let db = db::postgres::connect().await.map_err(internal_report)?;
    let file_storage = s3::connect().await;

    let contract_address = payload.contract_address;
    let token_id_low = payload.token_id_low;
    let token_id_high = payload.token_id_high;
    let token_id = CairoUint256::new(token_id_low, token_id_high);
    let key = contract_address.to_string()
        + "."
        + &token_id_low.to_string()
        + "."
        + &token_id_high.to_string();

    let uri = token::get_erc721_uri(contract_address, &BlockId::Tag(Latest), &rpc, token_id).await;
    let metadata = token::get_token_metadata(&uri).await.map_err(internal_report)?;

    let url_result = store_metadata(&key, &metadata, &file_storage).await;
    let url = match url_result {
        Some(url) => url,
        None => {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, "could not upload to s3".to_string()));
        }
    };

    sqlx::query!(
        r#"
    UPDATE token_metadata
    SET s3 = $1
    WHERE 
        contract_address = $2 AND
        token_id_low = $3 AND
        token_id_high = $4
"#,
        url,
        contract_address.to_string(),
        token_id_low.to_string(),
        token_id_high.to_string()
    )
    .execute(&db)
    .await
    .map_err(internal_error)?;

    Ok(StatusCode::OK)
}

fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

fn internal_report(err: Report) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
