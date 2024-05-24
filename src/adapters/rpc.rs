use futures_util::{SinkExt, StreamExt};
use http::Request;
use serde_json::json;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::handshake::client::generate_key;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream};

pub struct SolanaRpcClient {
    socket: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl SolanaRpcClient {
    pub async fn new(url: &str) -> Result<Self, String> {
        let parsed_url = url::Url::parse(url).unwrap();
        let request = Request::builder()
            .uri(parsed_url.to_string())
            .header("sec-websocket-key", generate_key())
            .header("host", "devnet-local.themetadao-org.workers.dev")
            .header("upgrade", "websocket")
            .header("connection", "upgrade")
            .header("sec-websocket-version", 13)
            .body(())
            .unwrap();
        let connection_res = connect_async(request).await;
        match connection_res {
            Ok((socket, _)) => Ok(SolanaRpcClient { socket }),
            Err(e) => Err(format!("error connecting RPC {:?}", e)),
        }
    }

    pub async fn on_account_change<F>(&mut self, acct: String, callback: F)
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
