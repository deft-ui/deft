use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

pub static RESOURCES: LazyLock<Resource> = LazyLock::new(|| Resource::new());

pub struct Resource {
    data: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl Resource {
    fn new() -> Resource {
        Resource {
            data: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn add(key: &str, value: Vec<u8>) {
        let data = RESOURCES.data.clone();
        let mut data = data.lock().unwrap();
        data.insert(key.to_string(), value);
    }

    pub fn read<R, F: FnOnce(&Vec<u8>) -> R>(key: &str, handler: F) -> Option<R> {
        let data = RESOURCES.data.clone();
        let data = data.lock().unwrap();
        data.get(&key.to_string()).map(|value| handler(value))
    }
}
