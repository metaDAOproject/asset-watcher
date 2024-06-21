use std::sync::Arc;

use crate::entities::token_accts::token_accts;
use crate::entities::token_accts::token_accts::dsl::*;
use crate::entities::token_accts::TokenAcct;
use crate::entities::token_accts::TokenAcctStatus;
use crate::entities::token_accts::WatchTokenBalancePayload;
use crate::entities::token_accts::WatchTokenBalanceResponse;
use deadpool::managed::Object;
use deadpool_diesel::Manager;
use diesel::prelude::*;
use diesel::update;
use diesel::PgConnection;
use warp::{reject::Reject, Reply};

#[derive(Debug)]
struct ParseError;
impl Reject for ParseError {}

pub async fn handler(
    reply_with_status: warp::reply::WithStatus<&'static str>,
    message: WatchTokenBalancePayload,
    conn_manager: Arc<Object<Manager<PgConnection>>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let response = reply_with_status.into_response();
    if !response.status().is_success() {
        return Ok(warp::reply::with_status(
            warp::reply::json(&WatchTokenBalanceResponse {
                message: "unsuccessful response status".to_string(),
            }),
            response.status(),
        ));
    }

    let token_acct_pubkey = message.token_acct.clone();
    let token_acct_clone = token_acct_pubkey.clone();

    let res = conn_manager
        .interact(move |db| {
            update_token_acct_with_status(token_acct_clone, TokenAcctStatus::Watching, db)
        })
        .await;

    match res {
        Ok(_) => Ok(warp::reply::with_status(
            warp::reply::json(&WatchTokenBalanceResponse {
                message: format!(
                    "updated token acct to {:?} status: {}",
                    status,
                    token_acct_pubkey.to_string()
                ),
            }),
            warp::http::StatusCode::OK,
        )),
        Err(e) => Ok(warp::reply::with_status(
            warp::reply::json(&WatchTokenBalanceResponse {
                message: format!(
                    "error updating token acct [{}] to {:?} status: {}",
                    token_acct_pubkey.to_string(),
                    status,
                    e
                ),
            }),
            warp::http::StatusCode::BAD_REQUEST,
        )),
    }
}

fn update_token_acct_with_status(
    token_acct_pubkey: String,
    token_acct_status: TokenAcctStatus,
    db: &mut PgConnection,
) -> Result<TokenAcct, diesel::result::Error> {
    update(token_accts::table.filter(token_accts::token_acct.eq(token_acct_pubkey.to_string())))
        .set(token_accts::dsl::status.eq(token_acct_status.clone()))
        .get_result::<TokenAcct>(db)
}
