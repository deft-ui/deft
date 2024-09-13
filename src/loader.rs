use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use anyhow::anyhow;
use quick_js::loader::JsModuleLoader;

pub struct RemoteModuleLoader {
    base: String,
}

impl RemoteModuleLoader {
    pub fn new(base: String) -> Self {
        Self {
            base,
        }
    }
}

impl JsModuleLoader for RemoteModuleLoader {
    fn load(&self, module_name: &str) -> Result<String, Error> {
        let url = if module_name.starts_with("http://") || module_name.starts_with("https://") {
            module_name.to_string()
        } else {
            format!("{}{}", self.base, module_name)
        };
        let body = reqwest::blocking::get(&url).map_err(|e| Error::new(ErrorKind::Other, e))?
            .text().map_err(|e| Error::new(ErrorKind::Other, e))?;
        Ok(body)
    }
}

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
    fn load(&self, module_name: &str) -> Result<String, Error> {
        match self.sources.get(module_name) {
            None => Err(Error::new(ErrorKind::NotFound, anyhow!("Not found"))),
            Some(s) => Ok(s.to_string())
        }
    }
}