use napi::bindgen_prelude::*;
use napi::{JsObject, JsUnknown, ValueType, Env, JsBuffer, JsFunction, JsString, KeyCollectionMode, KeyFilter, KeyConversion};
use napi_derive::napi;
use rustc_hash::FxHashMap;
use std::cell::RefCell;

#[derive(Clone)]
pub struct FieldDef {
    pub id: u32,
    pub name: String,
    pub field_type: String, // "string", "uint32", "message", etc.
    pub repeated: bool,
    pub required: bool, // Add required field
    pub is_map: bool,
    pub key_type: Option<String>,
    pub nested_type: Option<Box<NativeType>>, // For nested messages
}

#[napi]
#[derive(Clone)]
pub struct NativeType {
    fields: Vec<FieldDef>,
    name_to_index: FxHashMap<String, usize>,
}

#[napi]
impl NativeType {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            fields: Vec::new(),
            name_to_index: FxHashMap::default(),
        }
    }

    #[napi]
    pub fn add_field(&mut self, name: String, id: u32, field_type: String, repeated: bool, required: bool, is_map: bool, key_type: Option<String>, nested: Option<&NativeType>) -> napi::Result<()> {
        self.name_to_index.insert(name.clone(), self.fields.len());
        self.fields.push(FieldDef {
            id,
            name,
            field_type,
            repeated,
            required,
            is_map,
            key_type,
            nested_type: nested.map(|n| Box::new(n.clone())),
        });
        Ok(())
    }

    #[napi]
    pub fn encode(&self, env: Env, obj: JsObject) -> napi::Result<JsBuffer> {
        let mut buf = Vec::with_capacity(1024);
        self.encode_inner(&env, &obj, &mut buf)?;
        env.create_buffer_with_data(buf).map(|b| b.into_raw())
    }

    fn encode_inner(&self, env: &Env, obj: &JsObject, buf: &mut Vec<u8>) -> napi::Result<()> {
        // Optimization: Batch get own property names to reduce N-API calls for sparse objects
        // Use KeyCollectionMode::OwnOnly to let V8 filter prototype properties efficiently
        let own_props_names = obj.get_all_property_names(
            KeyCollectionMode::OwnOnly,
            KeyFilter::Enumerable,
            KeyConversion::NumbersToStrings
        )?;
        let len = own_props_names.get_array_length()?;
        
        thread_local! {
            static FIELD_VALUES_POOL: RefCell<Vec<Vec<Option<JsUnknown>>>> = RefCell::new(Vec::with_capacity(16));
        }

        FIELD_VALUES_POOL.with(|pool_cell| {
            // Borrow the pool to get a vector
            let mut pool = pool_cell.borrow_mut();
            let mut field_values = pool.pop().unwrap_or_else(|| Vec::with_capacity(self.fields.len()));
            
            // Drop the pool borrow so recursive calls can access it
            drop(pool);

            // Ensure capacity
            while field_values.len() < self.fields.len() {
                field_values.push(None);
            }
            
            // Reset used slots (only up to fields.len(), though vector might be larger)
            for i in 0..self.fields.len() {
                field_values[i] = None;
            }
        
            // ... (Logic to populate field_values) ...
            let result = (|| -> napi::Result<()> {
                for i in 0..len {
                     let key: JsUnknown = own_props_names.get_element(i)?;
                     if key.get_type()? == ValueType::String {
                         let key_str: JsString = unsafe { key.cast() };
                         let utf8 = key_str.into_utf8()?;
                         let s = utf8.as_str()?;
                         
                         if let Some(&idx) = self.name_to_index.get(s) {
                             let val: JsUnknown = obj.get_property(key)?;
                             field_values[idx] = Some(val);
                         }
                     }
                }

                for (i, field) in self.fields.iter().enumerate() {
                    let val_opt = &field_values[i];
                    
                    if val_opt.is_none() {
                        if field.required {
                            return Err(napi::Error::from_reason(format!("missing required '{}'", field.name)));
                        }
                        continue;
                    }

                    let val = val_opt.as_ref().unwrap();
                    let val_type = val.get_type()?;
                    
                    if val_type == ValueType::Undefined || val_type == ValueType::Null {
                        if field.required {
                            return Err(napi::Error::from_reason(format!("missing required '{}'", field.name)));
                        }
                        continue;
                    }

                    if field.is_map {
                            let map_obj: JsObject = unsafe { val.cast() };
                            let keys = map_obj.get_property_names()?;
                            let len = keys.get_array_length()?;
                            for i in 0..len {
                                let key_js: JsUnknown = keys.get_element(i)?;
                                let val_js: JsUnknown = map_obj.get_property(&key_js)?;
                                self.encode_map_entry(env, buf, field, key_js, val_js)?;
                            }
                        } else if field.repeated {
                            if val.is_array()? {
                                let arr: JsObject = unsafe { val.cast() };
                                let len = arr.get_array_length()?;
                                for i in 0..len {
                                    let item: JsUnknown = arr.get_element(i)?;
                                    self.encode_value(env, buf, field, item)?;
                                }
                            }
                        } else {
                            let val_cloned: JsUnknown = unsafe { val.cast() };
                            self.encode_value(env, buf, field, val_cloned)?;
                        }
                }
                Ok(())
            })();

            // Return vector to pool
            pool_cell.borrow_mut().push(field_values);
            
            result
        })
    }

    fn encode_map_entry(&self, env: &Env, buf: &mut Vec<u8>, field: &FieldDef, key_js: JsUnknown, val_js: JsUnknown) -> napi::Result<()> {
        let mut entry_buf = Vec::new();
        
        // Key (Field 1)
        let key_def = FieldDef {
            id: 1,
            name: "key".to_string(),
            field_type: field.key_type.clone().unwrap_or("string".to_string()),
            repeated: false,
            required: false,
            is_map: false,
            key_type: None,
            nested_type: None,
        };
        self.encode_value(env, &mut entry_buf, &key_def, key_js)?;
        
        // Value (Field 2)
        let val_def = FieldDef {
            id: 2,
            name: "value".to_string(),
            field_type: field.field_type.clone(),
            repeated: false,
            required: false,
            is_map: false,
            key_type: None,
            nested_type: field.nested_type.clone(),
        };
        self.encode_value(env, &mut entry_buf, &val_def, val_js)?;
        
        // Write Tag (Field ID, WireType 2)
        let tag = (field.id << 3) | 2;
        write_varint32_fast(buf, tag);
        write_varint32_fast(buf, entry_buf.len() as u32);
        buf.extend_from_slice(&entry_buf);
        Ok(())
    }

    fn encode_value(&self, env: &Env, buf: &mut Vec<u8>, field: &FieldDef, val: JsUnknown) -> napi::Result<()> {
        // Write Tag
        let wire_type = match field.field_type.as_str() {
            "string" | "bytes" | "message" => 2,
            "uint32" | "int32" | "int64" | "bool" | "sint32" | "sint64" => 0,
            "float" | "fixed32" | "sfixed32" => 5,
            "double" | "fixed64" | "sfixed64" => 1,
            "group" => 3,
            _ => 2, // Default to LD
        };
        let tag = (field.id << 3) | wire_type;

        match field.field_type.as_str() {
            "string" => {
                write_varint32_fast(buf, tag);
                let js_str: napi::JsString = val.try_into()?;
                let utf8 = js_str.into_utf8()?;
                let bytes = utf8.as_slice();
                write_varint32_fast(buf, bytes.len() as u32);
                buf.extend_from_slice(bytes);
            },
            "bytes" => {
                write_varint32_fast(buf, tag);
                if val.is_buffer()? {
                    let js_buf: JsBuffer = unsafe { val.cast() };
                    let value = js_buf.into_value()?;
                    let slice = value.as_ref();
                    write_varint32_fast(buf, slice.len() as u32);
                    buf.extend_from_slice(slice);
                } else {
                    eprintln!("Field {} is bytes but value is not buffer. Type: {:?}", field.name, val.get_type()?);
                }
            },
            "uint32" => {
                write_varint32_fast(buf, tag);
                let num: napi::JsNumber = val.try_into()?;
                let v: u32 = num.get_uint32()?;
                write_varint32_fast(buf, v);
            },
            "int32" => {
                write_varint32_fast(buf, tag);
                let num: napi::JsNumber = val.try_into()?;
                let v: i32 = num.get_int32()?;
                write_varint64_fast(buf, v as i64 as u64);
            },
            "int64" | "sint64" => {
                write_varint32_fast(buf, tag);
                let val_type = val.get_type()?;
                let v: i64 = if val_type == ValueType::BigInt {
                    let bigint = unsafe { val.cast::<napi::JsBigInt>() };
                    bigint.try_into()?
                } else if val_type == ValueType::Number {
                    let num: napi::JsNumber = val.try_into()?;
                    num.get_int64()?
                } else if val_type == ValueType::String {
                    let js_str: napi::JsString = val.try_into()?;
                    let s = js_str.into_utf8()?.into_owned()?;
                    s.parse::<i64>().unwrap_or(0)
                } else if val_type == ValueType::Object {
                    let obj: JsObject = unsafe { val.cast() };
                    let low: i32 = obj.get_named_property("low")?;
                    let high: i32 = obj.get_named_property("high")?;
                    ((high as i64) << 32) | (low as u32 as i64)
                } else {
                    0
                };
                
                if field.field_type == "sint64" {
                    let encoded = ((v << 1) ^ (v >> 63)) as u64;
                    write_varint64_fast(buf, encoded);
                } else {
                    write_varint64_fast(buf, v as u64);
                }
            },
            "sint32" => {
                write_varint32_fast(buf, tag);
                let num: napi::JsNumber = val.try_into()?;
                let v: i32 = num.get_int32()?;
                let encoded = ((v << 1) ^ (v >> 31)) as u32;
                write_varint32_fast(buf, encoded);
            },
            "float" | "fixed32" | "sfixed32" => {
                write_varint32_fast(buf, tag);
                let num: napi::JsNumber = val.try_into()?;
                if field.field_type == "float" {
                    let v: f64 = num.get_double()?;
                    buf.extend_from_slice(&(v as f32).to_le_bytes());
                } else {
                    let v: u32 = num.get_uint32()?;
                    buf.extend_from_slice(&v.to_le_bytes());
                }
            },
            "double" | "fixed64" | "sfixed64" => {
                write_varint32_fast(buf, tag);
                if field.field_type == "double" {
                    let num: napi::JsNumber = val.try_into()?;
                    let v: f64 = num.get_double()?;
                    buf.extend_from_slice(&v.to_le_bytes());
                } else {
                    // Handle fixed64/sfixed64 (8 bytes)
                    let val_type = val.get_type()?;
                    let v: u64 = if val_type == ValueType::BigInt {
                        let bigint = unsafe { val.cast::<napi::JsBigInt>() };
                        let (val, _lossless) = bigint.get_u64()?;
                        val
                    } else if val_type == ValueType::Number {
                        let num: napi::JsNumber = val.try_into()?;
                        num.get_int64()? as u64
                    } else if val_type == ValueType::String {
                        let js_str: napi::JsString = val.try_into()?;
                        let s = js_str.into_utf8()?.into_owned()?;
                        if field.field_type == "sfixed64" {
                            s.parse::<i64>().unwrap_or(0) as u64
                        } else {
                            s.parse::<u64>().unwrap_or(0)
                        }
                    } else if val_type == ValueType::Object {
                        let obj: JsObject = unsafe { val.cast() };
                        let low: i32 = obj.get_named_property("low")?;
                        let high: i32 = obj.get_named_property("high")?;
                        ((high as i64) << 32) as u64 | (low as u32 as u64)
                    } else {
                        0
                    };
                    buf.extend_from_slice(&v.to_le_bytes());
                }
            },
            "bool" => {
                write_varint32_fast(buf, tag);
                let b: napi::JsBoolean = val.try_into()?;
                let v = b.get_value()?;
                buf.push(if v { 1 } else { 0 });
            },
            "message" => {
                if let Some(nested) = &field.nested_type {
                    write_varint32_fast(buf, tag);
                    let mut nested_buf = Vec::new();
                    let nested_obj: JsObject = val.try_into()?;
                    nested.encode_inner(env, &nested_obj, &mut nested_buf)?;
                    
                    write_varint32_fast(buf, nested_buf.len() as u32);
                    buf.extend_from_slice(&nested_buf);
                }
            },
            "group" => {
                if let Some(nested) = &field.nested_type {
                    write_varint32_fast(buf, tag); // StartGroup
                    let nested_obj: JsObject = val.try_into()?;
                    nested.encode_inner(env, &nested_obj, buf)?; // Write content directly
                    
                    let end_tag = (field.id << 3) | 4; // EndGroup
                    write_varint32_fast(buf, end_tag);
                }
            }
            _ => {}
        }
        Ok(())
    }
}

#[inline(always)]
fn write_varint32_fast(buf: &mut Vec<u8>, mut value: u32) {
    // Reserve space to avoid multiple checks
    buf.reserve(5);
    unsafe {
        let mut ptr = buf.as_mut_ptr().add(buf.len());
        let mut len = 0;
        loop {
            if value > 0x7F {
                *ptr = (value as u8 & 0x7F) | 0x80;
                ptr = ptr.add(1);
                len += 1;
                value >>= 7;
            } else {
                *ptr = value as u8;
                len += 1;
                break;
            }
        }
        buf.set_len(buf.len() + len);
    }
}

#[inline(always)]
fn write_varint64_fast(buf: &mut Vec<u8>, mut value: u64) {
    buf.reserve(10);
    unsafe {
        let mut ptr = buf.as_mut_ptr().add(buf.len());
        let mut len = 0;
        loop {
            if value > 0x7F {
                *ptr = (value as u8 & 0x7F) | 0x80;
                ptr = ptr.add(1);
                len += 1;
                value >>= 7;
            } else {
                *ptr = value as u8;
                len += 1;
                break;
            }
        }
        buf.set_len(buf.len() + len);
    }
}
