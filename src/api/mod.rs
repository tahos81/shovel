use crate::common::types::CairoUint256;
use crate::db;
use crate::file_storage::metadata::store_metadata;
use crate::file_storage::*;
use crate::rpc;
use crate::rpc::metadata::token;
use axum::routing::post;
use axum::routing::Router;
use axum::Json;
use reqwest::StatusCode;
use serde::Deserialize;
use serde::Serialize;
use starknet::core::types::FieldElement;
use starknet::providers::jsonrpc::models::BlockId;
use starknet::providers::jsonrpc::models::BlockTag::Latest;

#[derive(Debug, Serialize, Deserialize)]
struct Payload {
    contract_address: String,
    token_id_low: String,
    token_id_high: String,
}

fn get_app() -> Router<()> {
    Router::new().route("/refresh", post(handler))
}

async fn handler(Json(payload): Json<Payload>) {
    let rpc = rpc::connect().unwrap();
    let db = db::postgres::connect().await.unwrap();
    let file_storage = s3::connect().await;
    let contract_address = FieldElement::from_hex_be(&payload.contract_address).unwrap();
    let token_id_low = payload.token_id_low;
    let token_id_high = payload.token_id_high;
    let token_id = CairoUint256::new(
        FieldElement::from_dec_str(&token_id_low).unwrap(),
        FieldElement::from_dec_str(&token_id_high).unwrap(),
    );
    let key = payload.contract_address + &token_id_low + &token_id_high;
    let uri = token::get_erc721_uri(contract_address, &BlockId::Tag(Latest), &rpc, token_id).await;
    let metadata = token::get_token_metadata(&uri).await.unwrap();
    let url = store_metadata(&key, &metadata, &file_storage).await.unwrap();
    let erc721_id = sqlx::query!(
        r#"
        SELECT id
        FROM erc721_data
        WHERE
            contract_address = $1 AND
            token_id_low = $2 AND
            token_id_high = $3
        "#,
        contract_address.to_string(),
        token_id_low,
        token_id_high,
    )
    .fetch_one(&db)
    .await
    .unwrap();

    let erc721_id = erc721_id.id;
    sqlx::query!(
        r#"
    UPDATE token_metadata
    SET s3 = $1
    WHERE id = $2
"#,
        url,
        erc721_id,
    )
    .execute(&db)
    .await
    .unwrap();
}
