use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
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
use serde::Serialize;
use crate::{js_value, js_weak_value};
use crate::js::{JsError, ToJsValue};

thread_local! {
    pub static NEXT_ID: RefCell<u64> = RefCell::new(0);
    pub static CONNECTIONS: RefCell<HashMap<u64, WsConnection>> = RefCell::new(HashMap::new());
}


type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[mrc_object]
pub struct WsConnection {
    id: u64,
    writer: Arc<Mutex<SplitSink<WsStream, Message>>>,
    reader: Arc<Mutex<SplitStream<WsStream>>>,
}


unsafe impl Send for WsConnection {}
unsafe impl Sync for WsConnection {}

js_weak_value!(WsConnection, WsConnectionWeak);

#[js_methods]
impl WsConnection {

    #[js_func]
    pub async fn connect(url: String) -> Result<WsConnection, JsError> {
        let id = NEXT_ID.with_borrow_mut(|id| {
            *id += 1;
            *id - 1
        });
        let (mut socket, _) = connect_async(url).await
            .map_err(|e| io::Error::new(ErrorKind::Other, e))?;
        let (writer, reader) = socket.split();
        let ws_conn = WsConnectionData {
            id,
            reader: Arc::new(Mutex::new(reader)),
            writer: Arc::new(Mutex::new(writer)),
        }.to_ref();
        CONNECTIONS.with_borrow_mut(|mut map| {
            map.insert(id, ws_conn.clone());
        });
        Ok(ws_conn)
    }

    #[js_func]
    pub fn close(&self) {
        CONNECTIONS.with_borrow_mut(|map| {
            map.remove(&self.id);
        });
    }

    #[js_func]
    pub async fn read(&self) -> Result<(String, JsValue), JsError> {
        let mut reader = self.inner.reader.lock().await;
        if let Some(result) = reader.next().await {
            let msg = result?;
            let (ty, data) = match msg {
                Message::Text(v) => ("text", v.to_js_value()?),
                Message::Binary(v) => ("binary", v.to_js_value()?),
                Message::Ping(v) => ("ping", v.to_js_value()?),
                Message::Pong(v) => ("pong", v.to_js_value()?),
                Message::Close(_) => ("close", JsValue::Undefined),
                Message::Frame(_frame) => ("frame", _frame.into_data().to_js_value()?),
            };
            Ok((ty.to_string(), data))
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

