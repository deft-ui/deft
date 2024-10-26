#![allow(unused)]
use std::collections::HashMap;
use quick_js::JsValue;
use serde::de::{DeserializeSeed, Error, MapAccess, SeqAccess, Visitor};
use serde::Deserializer;
use serde_json::Error as JsError;
use crate::js::js_value_util::JsValueHelper;

pub struct JsDeserializer {
    pub value: JsValue,
}

pub struct ArrayParser {
    index: usize,
    value: Vec<JsValue>,
}

pub struct ObjectParse {
    index: usize,
    value: HashMap<String, JsValue>,
}

impl<'de> SeqAccess<'de> for ArrayParser {
    type Error = JsError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>
    {
        if let Some(el) = self.value.get(self.index) {
            self.index += 1;
            let js_des = JsDeserializer{value: el.clone()};
            T::deserialize(seed, js_des).map(Some)
        } else {
            Ok(None)
        }
    }
}

impl<'de> MapAccess<'de> for ObjectParse {
    type Error = JsError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>
    {
        if let Some(el) = self.value.iter().nth(self.index) {
            //self.index += 1;
            let js_des = JsDeserializer{value: JsValue::String(el.0.to_string())};
            K::deserialize(seed, js_des).map(Some)
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>
    {
        if let Some(el) = self.value.iter().nth(self.index) {
            self.index += 1;
            let js_des = JsDeserializer{value: el.1.clone()};
            V::deserialize(seed, js_des)
        } else {
            Err(Error::custom(""))
        }
    }
}



impl<'de> Deserializer<'de> for JsDeserializer {
    type Error = JsError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match &self.value {
            JsValue::Undefined => {self.deserialize_unit(visitor)}
            JsValue::Null => {self.deserialize_unit(visitor)}
            JsValue::Bool(b) => {self.deserialize_bool(visitor)}
            JsValue::Int(i) => {self.deserialize_i32(visitor)}
            JsValue::Float(f) => {self.deserialize_f64(visitor)}
            JsValue::String(s) => {self.deserialize_string(visitor)}
            JsValue::Array(a) => {self.deserialize_seq(visitor)}
            _ => unimplemented!()
            // JsValue::Object(_) => {}
            // JsValue::Raw(_) => {}
            // JsValue::Date(_) => {}
            // JsValue::BigInt(_) => {}
            // JsValue::__NonExhaustive => {}
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if let JsValue::Bool(b) = self.value {
            visitor.visit_bool(b)
        } else {
            Err(Error::custom(""))
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if let JsValue::Int(i) = self.value {
            visitor.visit_i8(i as i8)
        } else {
            Err(Error::custom(""))
        }
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if let JsValue::Int(i) = self.value {
            visitor.visit_i16(i as i16)
        } else {
            Err(Error::custom(""))
        }
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if let JsValue::Int(i) = self.value {
            visitor.visit_i32(i)
        } else {
            Err(Error::custom(""))
        }
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if let JsValue::Float(i) = self.value {
            visitor.visit_i64(i as i64)
        } else {
            Err(Error::custom(""))
        }
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if let JsValue::Int(i) = self.value {
            visitor.visit_u8(i as u8)
        } else {
            Err(Error::custom(""))
        }
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if let JsValue::Int(i) = self.value {
            visitor.visit_u16(i as u16)
        } else {
            Err(Error::custom(""))
        }
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if let JsValue::Int(i) = self.value {
            visitor.visit_u32(i as u32)
        } else if let JsValue::Float(f) = self.value {
            //TODO check over?
            visitor.visit_u32(f as u32)
        } else {
            Err(Error::custom(""))
        }
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if let JsValue::Int(i) = self.value {
            visitor.visit_u64(i as u64)
        } else {
            Err(Error::custom("deserialize u64 error"))
        }
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if let Some(i) = self.value.as_number() {
            visitor.visit_f32(i as f32)
        } else {
            Err(Error::custom("deserialize f32 error"))
        }
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if let Some(i) = self.value.as_number() {
            visitor.visit_f64(i)
        } else {
            Err(Error::custom("deserialize f64 error"))
        }
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if let JsValue::String(s) = self.value {
            visitor.visit_str(s.as_str())
        } else {
            Err(Error::custom("deserialize str error"))
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if let JsValue::String(s) = self.value {
            visitor.visit_string(s)
        } else {
            Err(Error::custom("deserialize string error"))
        }
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if let JsValue::Null = &self.value {
            visitor.visit_none()
        } else if let JsValue::Undefined = &self.value {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if let JsValue::Undefined = &self.value {
            visitor.visit_unit()
        } else if let JsValue::Null = &self.value {
            visitor.visit_unit()
        } else {
            Err(Error::custom(""))
        }
    }

    fn deserialize_unit_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_newtype_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if let JsValue::Array(a) = self.value {
            visitor.visit_seq(ArrayParser {
                index: 0,
                value: a
            })
        } else {
            Err(Error::custom("deserialize seq error"))
        }
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(self, name: &'static str, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if let JsValue::Object(map) = self.value {
            visitor.visit_map(ObjectParse {
                index: 0,
                value: map,
            })
        } else if let Some(map) = self.value.get_properties() {
            visitor.visit_map(ObjectParse {
                index: 0,
                value: map,
            })
        } else {
            Err(Error::custom("deserialize map error"))
        }
    }

    fn deserialize_struct<V>(self, name: &'static str, fields: &'static [&'static str], visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(self, name: &'static str, variants: &'static [&'static str], visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }
}