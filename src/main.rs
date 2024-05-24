#[macro_use]
extern crate diesel;
extern crate dotenv;

use adapters::rpc::SolanaRpcClient;
use diesel::prelude::*;
use dotenv::dotenv;
use serde_json::{json, Value};
use std::env;
use std::sync::Arc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;
mod entities;
use entities::token_accts::{token_accts::dsl::*, TokenAcct};
mod adapters;
mod services;
use futures_util::sink::SinkExt;
use futures_util::StreamExt;
use services::balances_handler::BalancesHandler;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let connection = &mut establish_connection();

    let rpc_endpoint = env::var("RPC_ENDPOINT").expect("RPC_ENDPOINT must be set");
    let rpc_client = SolanaRpcClient::new(&rpc_endpoint).await;
    // let arc_connection = Arc::new(*connection);
    // let balances_handler = BalancesHandler::new(arc_connection);

    let results = token_accts
        .load::<TokenAcct>(connection)
        .expect("Error loading token_accts");

    for record in results {
        let _ = tokio::spawn(rpc_client.on_account_change(&record.token_acct, |msg| {
            let parsed_msg: Value =
                serde_json::from_str(msg.as_str()).expect("Failed to parse JSON");
            let new_amount = parsed_msg["params"]["result"]["value"]["amount"]
                .as_i64()
                .expect("Failed to get amount");
            format!("{}", msg);
            // put the logic in here:
            diesel::update(token_accts.filter(token_acct.eq(record.token_acct)))
                .set(amount.eq(new_amount))
                .execute(&mut connection);
        }));
    }
}

pub fn establish_connection() -> PgConnection {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

async fn connect_to_sol_rpc(record: TokenAcct) {
    let url = "wss://solana-rpc-endpoint";
    let (mut socket, _) = connect_async(url).await.expect("Failed to connect");

    let message = serde_json::to_string(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "onAccountChange",
        "params": {
            "account": record.token_acct,
            "commitment": "finalized"
        }
    }))
    .unwrap();

    socket.send(Message::Text(message)).await.unwrap();

    while let Some(msg) = socket.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                println!("Received: {}", text);
            }
            Ok(Message::Binary(bin)) => {
                println!("Received binary data: {:?}", bin);
            }
            Err(e) => {
                eprintln!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }
}
