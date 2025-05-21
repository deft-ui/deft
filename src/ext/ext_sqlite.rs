use crate as deft;
use crate::js::js_engine::JsEngine;
use crate::js::{JsError, ToJsValue};
use crate::js_value;
use crate::task_executor::TaskExecutor;
use deft_macros::{js_methods, mrc_object};
use quick_js::JsValue;
use rusqlite::types::{Type, Value};
use rusqlite::{Connection, ToSql};
use std::vec;

#[mrc_object]
pub struct SqliteConn {
    task_executor: TaskExecutor<Session>,
}

struct Session {
    conn: Option<Connection>,
}

impl Session {
    pub fn ensure_opened_mut(&mut self) -> Result<&mut Connection, String> {
        match &mut self.conn {
            None => Err("No connection".to_string()),
            Some(conn) => Ok(conn),
        }
    }
}

js_value!(SqliteConn);

#[js_methods]
impl SqliteConn {
    #[js_func]
    pub fn create() -> Result<Self, JsError> {
        let task_executor = TaskExecutor::new(move || Session { conn: None });
        Ok(SqliteConnData { task_executor }.to_ref())
    }

    #[js_func]
    pub fn open(&self, path: String) -> Result<JsValue, JsError> {
        let (promise, resolver) = JsEngine::get().create_promise();
        self.task_executor.run(move |sess| {
            let conn = match Connection::open(path) {
                Ok(conn) => conn,
                Err(e) => {
                    resolver.reject(JsValue::String(format!("Failed to open sqlite, {}", e)));
                    return;
                }
            };
            sess.conn = Some(conn);
            resolver.resolve(JsValue::Undefined);
        });
        Ok(promise)
    }

    #[js_func]
    pub fn execute(&self, sql: String, params: Vec<JsValue>) -> Result<JsValue, JsError> {
        let mut sql_values = Vec::new();
        for p in params {
            sql_values.push(js_value_to_sql_value(p)?);
        }
        let (promise, resolver) = JsEngine::get().create_promise();
        self.task_executor
            .run(move |session| resolver.settle(execute(session, sql, sql_values)));
        Ok(promise)
    }

    #[js_func]
    pub fn query(&self, sql: String, params: Vec<JsValue>) -> Result<JsValue, JsError> {
        let mut sql_values = Vec::new();
        for p in params {
            sql_values.push(js_value_to_sql_value(p)?);
        }
        let (promise, resolver) = JsEngine::get().create_promise();
        self.task_executor.run(move |conn| {
            let r = query(conn, sql, sql_values).map_err(|e| format!("failed to query, {}", e));
            resolver.settle(r);
        });
        Ok(promise)
    }
}

fn execute(sess: &mut Session, sql: String, sql_values: Vec<Value>) -> Result<JsValue, String> {
    let conn = sess.ensure_opened_mut()?;
    let mut sql_params = Vec::with_capacity(sql_values.len());
    unsafe { sql_params.set_len(sql_values.len()) };
    let sql_params_slice = sql_params.as_mut_slice();
    for i in 0..sql_values.len() {
        sql_params_slice[i] = sql_values.get(i).unwrap() as &dyn ToSql;
    }
    conn.execute(&sql, &*sql_params_slice)
        .map(|r| JsValue::Int(r as i32))
        .map_err(|e| format!("failed to execute sql: {}", e))
}

fn query(sess: &mut Session, sql: String, sql_values: Vec<Value>) -> Result<JsValue, String> {
    let conn = sess.ensure_opened_mut()?;
    let mut sql_params = Vec::with_capacity(sql_values.len());
    unsafe { sql_params.set_len(sql_values.len()) };
    let sql_params_slice = sql_params.as_mut_slice();
    for i in 0..sql_values.len() {
        sql_params_slice[i] = sql_values.get(i).unwrap() as &dyn ToSql;
    }
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("Failed to prepare sql: {}", e))?;
    let columns_names: Vec<JsValue> = stmt
        .column_names()
        .iter()
        .map(|s| JsValue::String(s.to_string()))
        .collect();
    let mut rows = stmt
        .query(&*sql_params_slice)
        .map_err(|e| format!("Failed to query: {}", e))?;
    let mut list = Vec::new();
    loop {
        let row = match rows
            .next()
            .map_err(|e| format!("Failed to fetch row: {}", e))?
        {
            Some(row) => row,
            None => break,
        };
        let mut result = Vec::new();
        for i in 0..columns_names.len() {
            if let Ok(v) = row.get_ref(i) {
                let value_type = v.data_type();
                let js_value = match value_type {
                    Type::Null => JsValue::Null,
                    Type::Integer => JsValue::Float(v.as_i64().unwrap() as f64),
                    Type::Real => JsValue::Float(v.as_f64().unwrap()),
                    Type::Text => JsValue::String(v.as_str().unwrap().to_string()),
                    //TODO fix blob
                    Type::Blob => JsValue::String("[blob]".to_string()),
                };
                result.push(js_value);
            } else {
                break;
            }
        }
        list.push(
            result
                .to_js_value()
                .map_err(|e| format!("Failed to convert value, {}", e))?,
        );
    }
    Ok(JsValue::Array(vec![
        JsValue::Array(columns_names),
        JsValue::Array(list),
    ]))
}

fn js_value_to_sql_value(js_value: JsValue) -> Result<Value, JsError> {
    let v = match js_value {
        JsValue::Undefined => Value::Null,
        JsValue::Null => Value::Null,
        JsValue::Bool(b) => Value::Integer(if b { 1 } else { 0 }),
        JsValue::Int(i) => Value::Integer(i64::from(i)),
        JsValue::Float(f) => Value::Real(f),
        JsValue::String(s) => Value::Text(s),
        _ => return Err(JsError::from_str("unexpected value type")),
    };
    Ok(v)
}
