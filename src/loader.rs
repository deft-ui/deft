#[cfg(feature = "http")]
mod http_loader;

use anyhow::anyhow;
use quick_js::loader::JsModuleLoader;
use std::collections::HashMap;
use std::io::{Error, ErrorKind};

#[cfg(feature = "http")]
pub use crate::loader::http_loader::DevModuleLoader;

pub struct StaticModuleLoader {
    sources: HashMap<String, String>,
}

impl StaticModuleLoader {
    pub fn new() -> Self {
        StaticModuleLoader {
            sources: HashMap::new(),
        }
    }
    pub fn add_module(&mut self, module_name: String, source: String) {
        self.sources.insert(module_name, source);
    }
}

impl JsModuleLoader for StaticModuleLoader {
    fn load(&mut self, module_name: &str) -> Result<String, Error> {
        match self.sources.get(module_name) {
            None => Err(Error::new(ErrorKind::NotFound, anyhow!("Not found"))),
            Some(s) => Ok(s.to_string()),
        }
    }
}
