use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream};

pub struct SolanaRpcClient {
    socket: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl SolanaRpcClient {
    pub async fn new(url: &str) -> Self {
        let (socket, _) = connect_async(url).await.expect("Failed to connect");
        SolanaRpcClient { socket }
    }

    pub async fn on_account_change<F>(&mut self, acct: &str, callback: F)
    where
        F: Fn(String) + Send + 'static,
    {
        let message = serde_json::to_string(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "onAccountChange",
            "params": {
                "account": acct,
                "commitment": "finalized"
            }
        }))
        .unwrap();

        self.socket.send(Message::Text(message)).await.unwrap();

        while let Some(msg) = self.socket.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    callback(text);
                }
                Ok(Message::Binary(bin)) => {
                    eprintln!("Received binary data: {:?}", bin);
                }
                Err(e) => {
                    eprintln!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }
    }
}
