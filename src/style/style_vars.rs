use std::collections::HashMap;

#[derive(Eq, PartialEq, Clone)]
pub struct StyleVars {
    data: HashMap<String, String>,
}

impl StyleVars {
    pub fn new() -> StyleVars {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn set(&mut self, key: &str, value: &str) {
        self.data.insert(key.to_string(), value.to_string());
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.data.get(key).map(|s| s.as_str())
    }

    pub fn merge(&mut self, other: StyleVars) {
        for (k, v) in other.data {
            self.data.insert(k, v);
        }
    }

    pub fn data(&self) -> &HashMap<String, String> {
        &self.data
    }
}
