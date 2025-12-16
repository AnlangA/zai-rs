use super::error::*;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::http::Request;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

type WssStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// WebSocket connection that provides split read/write handles
pub struct WssConnection {
    write_sink: futures_util::stream::SplitSink<WssStream, Message>,
    read_stream: futures_util::stream::SplitStream<WssStream>,
}

pub trait WssClient {
    type ApiUrl: AsRef<str>;
    type ApiKey: AsRef<str>;

    fn api_url(&self) -> &Self::ApiUrl;
    fn api_key(&self) -> &Self::ApiKey;

    fn connect(&self) -> impl std::future::Future<Output = ZaiResult<WssConnection>> + Send {
        let url = self.api_url().as_ref().to_owned();
        let key = self.api_key().as_ref().to_owned();

        async move {
            // 创建带有Authorization头部的请求
            let request = Request::builder()
                .uri(&url)
                .header("Authorization", format!("Bearer {}", key))
                .body(()) // WebSocket connection body should be empty
                .map_err(|e| ZaiError::from(e))?;

            let (ws_stream, response) = connect_async(request).await?;

            // 检查响应状态码，WebSocket成功升级应该是101
            let status = response.status().as_u16();
            if status != 101 {
                return Err(ZaiError::from_status_code(status, None));
            }

            // Split the WebSocket stream into read and write parts
            let (write_sink, read_stream) = ws_stream.split();

            Ok(WssConnection {
                write_sink,
                read_stream,
            })
        }
    }
}

impl WssConnection {
    /// Send a message through WebSocket
    pub async fn send(&mut self, msg: String) -> ZaiResult<()> {
        let message = Message::Text(msg);
        self.write_sink
            .send(message)
            .await
            .map_err(|e| ZaiError::websocket_error(0, format!("Failed to send message: {}", e)))
    }

    /// Read the next message from WebSocket
    pub async fn read(&mut self) -> ZaiResult<Option<String>> {
        match self.read_stream.next().await {
            Some(Ok(msg)) => {
                match msg {
                    Message::Text(text) => Ok(Some(text)),
                    Message::Binary(data) => {
                        // Convert binary data to string if possible, otherwise return base64
                        match String::from_utf8(data) {
                            Ok(text) => Ok(Some(text)),
                            Err(_) => Ok(Some("Binary data received".to_string())),
                        }
                    }
                    Message::Close(_) => Ok(None), // Connection closed gracefully
                    Message::Ping(_) | Message::Pong(_) => {
                        // Ping/Pong messages are handled automatically by tungstenite
                        Ok(None)
                    }
                    Message::Frame(frame) => {
                        // Handle raw WebSocket frames
                        // Use the frame's display format to get information
                        let frame_info = format!("Raw frame: {}", frame);
                        Ok(Some(frame_info))
                    }
                }
            }
            Some(Err(e)) => Err(ZaiError::websocket_error(
                0,
                format!("Failed to read message: {}", e),
            )),
            None => Ok(None), // Stream ended
        }
    }

    /// Close the WebSocket connection
    pub async fn close(&mut self) -> ZaiResult<()> {
        self.write_sink
            .close()
            .await
            .map_err(|e| ZaiError::websocket_error(0, format!("Failed to close connection: {}", e)))
    }
}
