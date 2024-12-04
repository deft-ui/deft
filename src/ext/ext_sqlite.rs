use crate as lento;
use std::path::Path;
use futures_util::FutureExt;
use lento_macros::{js_methods, mrc_object};
use quick_js::JsValue;
use rusqlite::{params, Connection, ToSql};
use rusqlite::types::{ToSqlOutput, Type, Value};
use crate::js::{JsError, ToJsValue};
use crate::{js_value, js_weak_value};

#[mrc_object]
pub struct SqliteConn {
    conn: Connection,
}

js_value!(SqliteConn);


#[js_methods]
impl SqliteConn {

    #[js_func]
    pub fn open(path: String) -> Result<Self, JsError> {
        let conn = Connection::open(path)?;
        let conn = SqliteConnData { conn }.to_ref();
        Ok(conn)
    }

    #[js_func]
    pub fn execute(&self, sql: String, params: Vec<JsValue>) -> Result<usize, JsError> {
        let mut sql_values = Vec::new();
        for p in params {
            sql_values.push(js_value_to_sql_value(p)?);
        }
        let mut sql_params = Vec::with_capacity(sql_values.len());
        unsafe { sql_params.set_len(sql_values.len()) };
        let mut sql_params_slice = sql_params.as_mut_slice();
        for i in 0..sql_values.len() {
            sql_params_slice[i] = sql_values.get(i).unwrap() as &dyn ToSql;
        }
        Ok(self.conn.execute(&sql, &*sql_params_slice)?)
    }

    #[js_func]
    pub fn query(&self, sql: String, params: Vec<JsValue>) -> Result<(Vec<String>, Vec<JsValue>), JsError> {
        let mut sql_values = Vec::new();
        for p in params {
            sql_values.push(js_value_to_sql_value(p)?);
        }
        let mut sql_params = Vec::with_capacity(sql_values.len());
        unsafe { sql_params.set_len(sql_values.len()) };
        let mut sql_params_slice = sql_params.as_mut_slice();
        for i in 0..sql_values.len() {
            sql_params_slice[i] = sql_values.get(i).unwrap() as &dyn ToSql;
        }
        let mut stmt = self.conn.prepare(&sql)?;
        let mut columns_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
        let mut rows = stmt.query(&*sql_params_slice)?;
        let mut list = Vec::new();
        loop {
            let row = match rows.next()? {
                Some(row) => row,
                None => break,
            };
            let mut result = Vec::new();
            for i in 0..columns_names.len() {
                if let Ok(v) = row.get_ref(i) {
                    let value_type = v.data_type();
                    let js_value = match value_type {
                        Type::Null => JsValue::Null,
                        Type::Integer => JsValue::Float(v.as_i64()? as f64),
                        Type::Real => JsValue::Float(v.as_f64()?),
                        Type::Text => JsValue::String(v.as_str()?.to_string()),
                        //TODO fix blob
                        Type::Blob => JsValue::String("[blob]".to_string()),
                    };
                    result.push(js_value);
                } else {
                    break;
                }
            }
            list.push(result.to_js_value()?);
        }
        Ok((columns_names, list))
    }
}

fn js_value_to_sql_value(js_value: JsValue) -> Result<Value, JsError> {
    let v = match js_value {
        JsValue::Undefined => Value::Null,
        JsValue::Null => Value::Null,
        JsValue::Bool(b) => Value::Integer(if b { 1 } else { 0 }),
        JsValue::Int(i) => Value::Integer(i64::from(i)),
        JsValue::Float(f) => Value::Real(f),
        JsValue::String(s) => Value::Text(s),
        _ => return Err(JsError::from_str("unexpceted value type")),
    };
    Ok(v)
}