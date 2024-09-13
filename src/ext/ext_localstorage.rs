use anyhow::Error;
use lazy_static::lazy_static;
use crate::data_dir::get_data_path;
lazy_static! {
    static ref DB: sled::Db = {
        let dir = get_data_path("localstorage");
        sled::open(dir).unwrap()
    };
}

pub fn localstorage_set(key: String, value: String) -> Result<(), Error> {
    DB.insert(key, value.as_bytes())?;
    Ok(())
}

pub fn localstorage_get(key: String) -> Result<Option<String>, Error> {
    if let Some(v) = DB.get(key)? {
        Ok(Some(String::from_utf8(v.to_vec()).unwrap()))
    } else {
        Ok(None)
    }
}

pub fn localstorage_flush() -> Result<(), Error> {
    DB.flush()?;
    Ok(())
}