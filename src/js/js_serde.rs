#![allow(unused)]
use std::collections::HashMap;
use quick_js::JsValue;
use serde::{Serialize, Serializer};
use serde::ser::{SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple, SerializeTupleStruct, SerializeTupleVariant};
use serde_json::Error;

pub struct JsValueSerializer {

}

pub struct JsArraySerializer {
    serializer: JsValueSerializer,
    array: Vec<JsValue>,
}

pub struct JsObjectSerializer {
    pub serializer: JsValueSerializer,
    pub map: HashMap<String, JsValue>,
}

impl Serializer for JsValueSerializer {
    type Ok = JsValue;
    type Error = Error;
    type SerializeSeq = JsArraySerializer;
    type SerializeTuple = JsArraySerializer;
    type SerializeTupleStruct = JsArraySerializer;
    type SerializeTupleVariant = JsArraySerializer;
    type SerializeMap = JsObjectSerializer;
    type SerializeStruct = JsObjectSerializer;
    type SerializeStructVariant = JsObjectSerializer;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(JsValue::Bool(v))
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        Ok(JsValue::Int(v as i32))
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        Ok(JsValue::Int(v as i32))
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        Ok(JsValue::Int(v))
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        Ok(JsValue::Float(v as f64))
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        Ok(JsValue::Int(v as i32))
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        Ok(JsValue::Int(v as i32))
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        Ok(JsValue::Float(v as f64))
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        Ok(JsValue::Float(v as f64))
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        Ok(JsValue::Float(v as f64))
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        Ok(JsValue::Float(v))
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        Ok(JsValue::String(v.to_string()))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(JsValue::String(v.to_string()))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(JsValue::Null)
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(JsValue::Null)
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        Ok(JsValue::Null)
    }

    fn serialize_unit_variant(self, name: &'static str, variant_index: u32, variant: &'static str) -> Result<Self::Ok, Self::Error> {
        Ok(JsValue::String(variant.to_string()))
    }

    fn serialize_newtype_struct<T>(self, name: &'static str, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(self, name: &'static str, variant_index: u32, variant: &'static str, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize
    {
        let mut result: HashMap<String, JsValue> = HashMap::new();
        result.insert(variant.to_string(), value.serialize(self)?);
        todo!()
        // Ok(JsValue::Object(result))
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(JsArraySerializer{
            serializer: self,
            array: Vec::new(),
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(JsArraySerializer{
            serializer: self,
            array: Vec::new(),
        })
    }

    fn serialize_tuple_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(JsArraySerializer{
            serializer: self,
            array: Vec::new(),
        })
    }

    fn serialize_tuple_variant(self, name: &'static str, variant_index: u32, variant: &'static str, len: usize) -> Result<Self::SerializeTupleVariant, Self::Error> {
        todo!()
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(JsObjectSerializer {
            serializer: self,
            map: HashMap::new(),
        })
    }

    fn serialize_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(JsObjectSerializer {
            serializer: self,
            map: HashMap::new(),
        })
    }

    fn serialize_struct_variant(self, name: &'static str, variant_index: u32, variant: &'static str, len: usize) -> Result<Self::SerializeStructVariant, Self::Error> {
        todo!()
    }
}

impl SerializeSeq for JsArraySerializer {
    type Ok = JsValue;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize
    {
        self.array.push(value.serialize(JsValueSerializer {})?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(JsValue::Array(self.array))
    }
}

impl SerializeTuple for JsArraySerializer {
    type Ok = JsValue;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize
    {
        self.array.push(value.serialize(JsValueSerializer {})?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(JsValue::Array(self.array))
    }
}

impl SerializeTupleStruct for JsArraySerializer {
    type Ok = JsValue;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize
    {
        self.array.push(value.serialize(JsValueSerializer {})?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(JsValue::Array(self.array))
    }
}

impl SerializeTupleVariant for JsArraySerializer {
    type Ok = JsValue;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize
    {
        self.array.push(value.serialize(JsValueSerializer {})?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(JsValue::Array(self.array))
    }
}

impl SerializeMap for JsObjectSerializer {
    type Ok = JsValue;
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize
    {
        todo!()
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize
    {
        todo!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
}

impl SerializeStruct for JsObjectSerializer {
    type Ok = JsValue;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize
    {
        self.map.insert(key.to_string(), value.serialize(JsValueSerializer {})?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(JsValue::Object(self.map))
    }
}

impl SerializeStructVariant for JsObjectSerializer {
    type Ok = JsValue;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize
    {
        todo!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
}