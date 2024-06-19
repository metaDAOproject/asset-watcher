use std::env;
use std::str::FromStr;
use std::sync::Arc;

use crate::entities::token_accts::token_accts::dsl::*;
use crate::entities::token_accts::TokenAcct;
use crate::entities::token_accts::WatchTokenBalancePayload;
use crate::entrypoints::events;
use deadpool::managed::Object;
use deadpool_diesel::Manager;
use diesel::prelude::*;
use diesel::PgConnection;
use solana_client::nonblocking::pubsub_client::PubsubClient;
use solana_sdk::pubkey::Pubkey;
use tokio::task;
use warp::http::StatusCode;
use warp::reply::WithStatus;
use warp::{reject::Reject, Reply};

#[derive(Debug)]
struct ParseError;
impl Reject for ParseError {}

pub async fn handler(
    reply_with_status: warp::reply::WithStatus<&'static str>,
    message: WatchTokenBalancePayload,
    conn_manager: Arc<Object<Manager<PgConnection>>>,
    pub_sub_client: Arc<PubsubClient>,
) -> Result<WithStatus<&str>, warp::Rejection> {
    let response = reply_with_status.into_response();
    if !response.status().is_success() {
        return Ok(warp::reply::with_status("status error", response.status()));
    }

    let token_acct_pubkey = message.token_acct.clone();

    // Fetch the token account record from the database
    let token_acct_record = conn_manager
        .interact(move |conn| {
            token_accts
                .filter(token_acct.eq(&message.token_acct))
                .first::<TokenAcct>(conn)
        })
        .await;

    match token_acct_record {
        Ok(Ok(record_res)) => {
            task::spawn(async move {
                // we have to construct a fresh connection here because of the new thread
                let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
                let mut connection_val =
                    PgConnection::establish(&database_url).expect("could not establish connection");
                match Pubkey::from_str(&token_acct_pubkey) {
                    Ok(pubkey) => {
                        events::rpc_token_acct_updates::new_handler(
                            pub_sub_client,
                            &mut connection_val,
                            pubkey,
                            record_res,
                        )
                        .await
                    }
                    Err(e) => eprintln!("{:?}", e),
                }
            });
            // TODO: maybe implement a channel so that the new thread can let the requester know if there is a problem
            Ok(warp::reply::with_status(
                "successfully began watching token_acct",
                StatusCode::OK,
            ))
        }
        _ => Ok(warp::reply::with_status(
            "could not find token acct to watch",
            StatusCode::NOT_FOUND,
        )),
    }
}
