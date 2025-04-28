use std::io::{Error, ErrorKind};
use anyhow::anyhow;
use quick_js::loader::{FsJsModuleLoader, JsModuleLoader};

pub struct RemoteModuleLoader {
    base: Option<String>,
}

impl RemoteModuleLoader {
    pub fn new(base: Option<String>) -> Self {
        Self {
            base,
        }
    }
}

impl JsModuleLoader for RemoteModuleLoader {
    fn load(&mut self, module_name: &str) -> Result<String, Error> {
        let url = if module_name.starts_with("http://") || module_name.starts_with("https://") {
            module_name.to_string()
        } else if let Some(base) = &self.base {
            format!("{}/{}", base.trim_end_matches("/"), module_name.trim_start_matches("/"))
        } else {
            return Err(Error::new(ErrorKind::AddrNotAvailable, anyhow!("Failed to resolve module: {}", module_name)));
        };
        let body = reqwest::blocking::get(&url)
            .map_err(|e| Error::new(ErrorKind::Other, format!("Failed to request {:?}", e)))?
            .text()
            .map_err(|e| Error::new(ErrorKind::Other, format!("Failed to read response text: {:?}", e)))?;
        Ok(body)
    }
}

pub struct DevModuleLoader {
    is_first_load: bool,
    remote_module_loader: RemoteModuleLoader,
}

impl DevModuleLoader {
    pub fn new(base: Option<&str>) -> Self {
        let base = base.map(String::from);
        Self {
            is_first_load: true,
            remote_module_loader: RemoteModuleLoader::new(base),
        }
    }
}

impl JsModuleLoader for DevModuleLoader {
    fn load(&mut self, module_name: &str) -> Result<String, Error> {
        let is_first_load = self.is_first_load;
        self.is_first_load = false;
        let start_time = std::time::Instant::now();
        loop {
            let result = self.remote_module_loader.load(module_name);
            match result {
                Ok(source) => {
                    return Ok(source);
                }
                Err(err) => {
                    if is_first_load || start_time.elapsed() >= std::time::Duration::from_secs(60) {
                        return Err(err);
                    }
                    std::thread::sleep(std::time::Duration::from_millis(1000));
                }
            }
        }
    }
}

pub struct DefaultModuleLoader {
    remote_module_loader: Option<RemoteModuleLoader>,
    fs_module_loader: Option<FsJsModuleLoader>,
}

impl DefaultModuleLoader {

    pub fn new(allow_remote: bool) -> Self {
        let remote_module_loader = if allow_remote {
            Some(RemoteModuleLoader::new(None))
        } else {
            None
        };
        Self {
            remote_module_loader,
            fs_module_loader: None,
        }
    }

    pub fn set_fs_base(&mut self, dir: &str) {
        self.fs_module_loader = Some(FsJsModuleLoader::new(dir))
    }

}

impl JsModuleLoader for DefaultModuleLoader {
    fn load(&mut self, module_name: &str) -> Result<String, Error> {
        if let Some(fs_loader) = &mut self.fs_module_loader {
            if let Ok(module) = fs_loader.load(module_name) {
                return Ok(module)
            }
        }
        if let Some(remote_loader) = &mut self.remote_module_loader {
            return remote_loader.load(module_name);
        }
        Err(Error::new(ErrorKind::NotFound, "failed to load module"))
    }
}