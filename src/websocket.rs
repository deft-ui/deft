use std::collections::HashMap;
use std::io;
use std::io::ErrorKind;
use std::ops::{Deref, DerefMut};
use anyhow::anyhow;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::StreamExt;
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tokio_tungstenite::tungstenite::Message;
use crate::mrc::Mrc;

#[derive(Clone)]
pub struct WebSocketManager {
    inner: Mrc<WebSocketManagerInner>,
}

unsafe impl Send for WebSocketManager {}
unsafe impl Sync for WebSocketManager {}

impl WebSocketManager {
    pub fn new() -> Self {
        Self {
            inner: Mrc::new(WebSocketManagerInner::new()),
        }
    }
}

impl Deref for WebSocketManager {
    type Target = WebSocketManagerInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for WebSocketManager {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub struct WebSocketManagerInner {
    next_client_id: i32,
    clients: HashMap<i32, (SplitSink<WsStream, Message>, SplitStream<WsStream>)>,
}

impl WebSocketManagerInner {
    pub fn new() -> Self {
        Self {
            next_client_id: 1,
            clients: HashMap::new(),
        }
    }
    pub async fn create_connection(&mut self, url: &str) -> Result<i32, io::Error> {
        let id = self.next_client_id;
        self.next_client_id += 1;
        let (mut socket, _) = connect_async(url).await
            .map_err(|e| io::Error::new(ErrorKind::Other, e))?;
        let (writer, reader) = socket.split();
        self.clients.insert(id, (writer, reader));
        Ok(id)
    }

    pub async fn read_msg(&mut self, id: i32) -> Result<Message, io::Error> {
        //TODO no unwrap
        let (_, reader) = self.clients.get_mut(&id).unwrap();
        if let Some(result) = reader.next().await {
            result.map_err(|e| io::Error::new(ErrorKind::Other, e))
        } else {
            Err(io::Error::new(ErrorKind::Other, anyhow!("eof")))
        }
    }

}
