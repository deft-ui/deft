use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, LazyLock, Mutex};

static FONT_FAMILY_MAP: LazyLock<Arc<Mutex<HashMap<String, FontFamily>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(HashMap::new())));

#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct FontFamily {
    name: Arc<String>,
}

impl Serialize for FontFamily {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.name.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for FontFamily {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let name = String::deserialize(deserializer)?;
        Ok(FontFamily::new(&name))
    }
}

impl FontFamily {
    pub fn new(name: &str) -> FontFamily {
        let mut map = FONT_FAMILY_MAP.lock().unwrap();
        map.entry(name.to_string())
            .or_insert_with_key(|k| {
                let name = Arc::new(k.to_string());
                FontFamily { name }
            })
            .clone()
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }
}

#[derive(Default, Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct FontFamilies {
    list: Vec<FontFamily>,
}

impl FontFamilies {
    pub fn new(list: Vec<FontFamily>) -> FontFamilies {
        Self { list }
    }

    pub fn as_slice(&self) -> &[FontFamily] {
        self.list.as_slice()
    }

    pub fn append(&self, families: &FontFamilies) -> Self {
        let mut list = Vec::with_capacity(self.list.len() + families.list.len());
        let mut hash_set = HashSet::new();
        Self::add(&mut list, &mut hash_set, &self.list);
        Self::add(&mut list, &mut hash_set, &families.list);
        Self { list }
    }

    fn add<'a>(
        result: &mut Vec<FontFamily>,
        hash_set: &mut HashSet<&'a FontFamily>,
        sources: &'a Vec<FontFamily>,
    ) {
        for it in sources {
            if !hash_set.contains(it) {
                hash_set.insert(it);
            }
            result.push(it.clone());
        }
    }
}
