use std::cell::RefCell;
use std::io;
use std::io::ErrorKind;
use std::rc::Rc;
use std::sync::Arc;
use anyhow::{anyhow, Error};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::StreamExt;
use quick_js::{JsValue, ResourceValue};
use quick_js::JsValue::Resource;
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tokio_tungstenite::tungstenite::Message;
use tokio::sync::Mutex;
use crate::js::js_value_util::{FromJsValue2, ToJsValue2};

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub struct WebsocketInner {
    writer: SplitSink<WsStream, Message>,
    reader: SplitStream<WsStream>,
}

#[derive(Clone)]
pub struct Websocket {
    inner: Arc<Mutex<WebsocketInner>>,
}

impl FromJsValue2 for Websocket {
    fn from_js_value(client: JsValue) -> Result<Self, Error> {
        if let Some(ws) = client.as_resource(|w: &mut Websocket| w.clone()) {
            Ok(ws)
        } else {
            Err(anyhow!("invalid client"))
        }
    }
}

impl ToJsValue2 for Websocket {
    fn to_js_value(self) -> Result<JsValue, Error> {
        Ok(Resource(ResourceValue {
            resource: Rc::new(RefCell::new(self)),
        }))
    }
}

pub async fn ws_connect(url: String) -> Result<Websocket, Error> {
    let (mut socket, _) = connect_async(url).await
        .map_err(|e| io::Error::new(ErrorKind::Other, e))?;
    let (writer, reader) = socket.split();
    let inner = WebsocketInner {
        reader,
        writer,
    };
    Ok(Websocket { inner: Arc::new(Mutex::new(inner)) })
}

pub async fn ws_read(ws: Websocket) -> Result<JsValue, Error> {
    let mut inner = ws.inner.lock().await;
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
