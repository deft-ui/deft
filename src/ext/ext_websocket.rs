use std::cell::RefCell;
use std::io;
use std::io::ErrorKind;
use std::rc::Rc;
use std::sync::Arc;
use anyhow::{anyhow, Error};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::StreamExt;
use quick_js::{JsValue, ResourceValue};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tokio_tungstenite::tungstenite::Message;
use tokio::sync::Mutex;
use crate::define_ref_and_resource;
use crate::js::js_value_util::{FromJsValue, ToJsValue};

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

struct WsConnectionInner {
    writer: SplitSink<WsStream, Message>,
    reader: SplitStream<WsStream>,
}

pub struct WsConnection {
    inner: Arc<Mutex<WsConnectionInner>>,
}

define_ref_and_resource!(WsConnectionResource, WsConnection);
unsafe impl Send for WsConnectionResource {}
unsafe impl Sync for WsConnectionResource {}

pub async fn ws_connect(url: String) -> Result<WsConnectionResource, Error> {
    let (mut socket, _) = connect_async(url).await
        .map_err(|e| io::Error::new(ErrorKind::Other, e))?;
    let (writer, reader) = socket.split();
    let inner = WsConnectionInner {
        reader,
        writer,
    };
    let ws_conn = WsConnection { inner: Arc::new(Mutex::new(inner)) };
    Ok(WsConnectionResource::new(ws_conn))
}

pub async fn ws_read(ws: WsConnectionResource) -> Result<JsValue, Error> {
    let mut inner = ws.inner.inner.lock().await;
    if let Some(result) = inner.reader.next().await {
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
