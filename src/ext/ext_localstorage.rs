use crate as deft;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use anyhow::{anyhow, Error};
use deft_macros::js_methods;
use log::debug;
use crate::js::JsError;

thread_local! {
    static DB: RefCell<Option<KVStorage>> = RefCell::new(None);
}

#[allow(nonstandard_style)]
pub struct localstorage {}

#[js_methods]
impl localstorage {

    pub fn init(path: PathBuf) {
        DB.with_borrow_mut(move |db| {
            *db = Some(KVStorage::new(path));
        });
    }

    #[js_func]
    pub fn set(key: String, value: String) -> Result<(), JsError> {
        let db = Self::get_storage()?;
        db.set(key, value);
        Ok(())
    }

    #[js_func]
    pub fn get(key: String) -> Result<Option<String>, JsError> {
        let db = Self::get_storage()?;
        Ok(db.get(key))
    }

    pub fn cleanup() -> Result<(), JsError> {
        if let Ok(db) = Self::get_storage() {
            db.cleanup();
        }
        Ok(())
    }

    fn get_storage() -> Result<KVStorage, JsError> {
        DB.with(|db| {
            if let Some(db) = db.borrow().deref() {
                Ok(db.clone())
            } else {
                Err(JsError::from_str("localstorage is not enabled"))
            }
        })
    }

}

enum KVMsg {
    Write((String, String)),
    Cleanup,
}

#[derive(Clone)]
struct KVStorage {
    path: PathBuf,
    data: Arc<Mutex<HashMap<String, String>>>,
    sender: Sender<KVMsg>,
    write_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl KVStorage {
    fn new(dir: PathBuf) -> Self {
        let db = Self::open_db(&dir).expect("failed to open localstorage");
        let mut data = HashMap::new();
        for e in db.iter() {
            let (k, v) = e.unwrap();
            let k = String::from_utf8(k.to_vec()).unwrap();
            let v = String::from_utf8(v.to_vec()).unwrap();
            data.insert(k, v);
        }
        let (sender, receiver) = channel::<KVMsg>();
        let write_handle = {
            let dir = dir.clone();
            thread::spawn(move || {
                loop {
                    let mut list = Vec::new();
                    let mut stopped = false;
                    loop {
                        match receiver.recv_timeout(Duration::from_millis(1000)) {
                            Ok(e) => {
                                match e {
                                    KVMsg::Write((k, v)) => {
                                        list.push((k, v));
                                        if list.len() > 100 {
                                            break;
                                        }
                                    }
                                    KVMsg::Cleanup => {
                                        stopped = true;
                                        break;
                                    }
                                }
                            }
                            Err(_) => {
                                break;
                            }
                        }
                    }
                    if !list.is_empty() {
                        let db = Self::open_db(&dir).expect("failed to open localstorage");
                        for (k, v) in list.iter() {
                            db.insert(k, v.as_bytes()).unwrap();
                        }
                        db.flush().expect("failed to flush localstorage");
                        debug!("localstorage flushed");
                        list.clear();
                    }
                    if stopped {
                        break;
                    }
                }
            })
        };

        Self {
            path: dir,
            data: Arc::new(Mutex::new(data)),
            sender,
            write_handle: Arc::new(Mutex::new(Some(write_handle))),
        }
    }

    fn set(&self, key: String, value: String) {
        let mut data = self.data.lock().expect("failed to lock localstorage");
        data.insert(key.clone(), value.clone());
        self.sender.send(KVMsg::Write((key, value))).unwrap();
    }

    fn get(&self, key: String) -> Option<String> {
        let data = self.data.lock().expect("failed to lock localstorage");
        data.get(&key).cloned()
    }

    fn cleanup(&self) {
        self.sender.send(KVMsg::Cleanup).unwrap();
        let write_handle = self.write_handle.clone();
        let mut write_handle = write_handle.lock().unwrap();
        if let Some(handle) = write_handle.take() {
            handle.join().unwrap();
        }
    }

    fn open_db(path: &PathBuf) -> Result<sled::Db, Error> {
        for _ in 0..30 {
            if let Ok(db) = sled::open(path) {
                return Ok(db);
            }
            thread::sleep(Duration::from_millis(100));
        }
        Err(anyhow!("failed to open localstorage"))
    }

}