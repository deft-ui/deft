use crate as lento;
use std::io;
use std::io::ErrorKind;
use std::sync::Arc;
use anyhow::{Error};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use quick_js::{JsValue};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tokio_tungstenite::tungstenite::Message;
use tokio::sync::Mutex;
use lento_macros::{js_methods, mrc_object};
use crate::{js_value};
use crate::js::JsError;

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[mrc_object]
pub struct WsConnection {
    writer: Arc<Mutex<SplitSink<WsStream, Message>>>,
    reader: Arc<Mutex<SplitStream<WsStream>>>,
}


unsafe impl Send for WsConnectionRef {}
unsafe impl Sync for WsConnectionRef {}

js_value!(WsConnectionRef);

#[js_methods]
impl WsConnectionRef {

    #[js_func]
    pub async fn connect(url: String) -> Result<WsConnectionRef, JsError> {
        let (mut socket, _) = connect_async(url).await
            .map_err(|e| io::Error::new(ErrorKind::Other, e))?;
        let (writer, reader) = socket.split();
        let ws_conn = WsConnection {
            reader: Arc::new(Mutex::new(reader)),
            writer: Arc::new(Mutex::new(writer)),
        };
        Ok(ws_conn.to_ref())
    }

    #[js_func]
    pub async fn read(&self) -> Result<JsValue, JsError> {
        let mut reader = self.inner.reader.lock().await;
        if let Some(result) = reader.next().await {
            let msg = result?;
            let value = match msg {
                Message::Text(v) => JsValue::String(v),
                Message::Binary(v) => {
                    JsValue::Array(v.into_iter().map(|e| JsValue::Int(e as i32)).collect())
                }
                Message::Ping(_) => JsValue::Undefined,
                Message::Pong(_) => JsValue::Undefined,
                Message::Close(_) => JsValue::Bool(false),
                Message::Frame(_frame) => JsValue::Undefined, //TODO handling Frame?
            };
            Ok(value)
        } else {
            Err(JsError::from_str("connection closed"))
        }
    }

    #[js_func]
    pub async fn send_str(&self, data: String) -> Result<JsValue, Error> {
        let t = data.to_string();
        let mut writer = self.inner.writer.lock().await;
        writer.send(Message::Text(data)).await?;
        Ok(JsValue::Undefined)
    }

}

