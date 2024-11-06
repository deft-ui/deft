use std::io;
use std::io::ErrorKind;
use std::sync::Arc;
use anyhow::{anyhow, Error};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use quick_js::{JsValue};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tokio_tungstenite::tungstenite::Message;
use tokio::sync::Mutex;
use crate::define_ref_and_resource;

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub struct WsConnection {
    writer: Arc<Mutex<SplitSink<WsStream, Message>>>,
    reader: Arc<Mutex<SplitStream<WsStream>>>,
}

define_ref_and_resource!(WsConnectionResource, WsConnection);
unsafe impl Send for WsConnectionResource {}
unsafe impl Sync for WsConnectionResource {}

pub async fn ws_connect(url: String) -> Result<WsConnectionResource, Error> {
    let (mut socket, _) = connect_async(url).await
        .map_err(|e| io::Error::new(ErrorKind::Other, e))?;
    let (writer, reader) = socket.split();
    let ws_conn = WsConnection {
        reader: Arc::new(Mutex::new(reader)),
        writer: Arc::new(Mutex::new(writer)),
    };
    Ok(WsConnectionResource::new(ws_conn))
}

pub async fn ws_read(ws: WsConnectionResource) -> Result<JsValue, Error> {
    let mut reader = ws.inner.reader.lock().await;
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
        Err(anyhow!("eof"))
    }
}

pub async fn ws_send_str(ws: WsConnectionResource, data: String) -> Result<JsValue, Error> {
    let t = data.to_string();
    let mut writer = ws.inner.writer.lock().await;
    writer.send(Message::Text(data)).await?;
    Ok(JsValue::Undefined)
}
