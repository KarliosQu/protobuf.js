use napi::bindgen_prelude::*;
use napi::{JsObject, JsUnknown, ValueType, Env, JsBuffer, JsTypedArray, JsFunction, JsString, KeyCollectionMode, KeyFilter, KeyConversion, JsNumber, JsBigInt, JsBoolean};
use napi_derive::napi;
use rustc_hash::FxHashMap;
use std::cell::RefCell;
use crate::pool::{BufferPool, BUFFER_POOL};

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
    pub oneof_name: Option<String>,
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
    pub fn encode_hybrid(&self, env: Env, ops: Int32Array, refs: JsObject) -> napi::Result<JsBuffer> {
        let mut buf = Vec::with_capacity(1024);
        
        // ops format: [FieldIndex, OpCode, ValueOrRefIndex]
        // OpCodes:
        // 0: Varint32 (Value is immediate)
        // 1: Varint64 / Long (Value is RefIndex)
        // 2: String (Value is RefIndex)
        // 3: Bytes (Value is RefIndex)
        // 4: Message (Value is RefIndex)
        // 5: Fixed32 (Value is immediate)
        // 6: Fixed64 (Value is RefIndex)
        // 7: Float (Value is RefIndex)
        // 8: Double (Value is RefIndex)
        // 9: Bool (Value is immediate)
        
        let ops_slice = ops.as_ref();
        let mut i = 0;
        while i < ops_slice.len() {
            let field_idx = ops_slice[i] as usize;
            let op_code = ops_slice[i+1];
            let val_or_ref = ops_slice[i+2];
            i += 3;

            if field_idx >= self.fields.len() {
                continue;
            }
            let field = &self.fields[field_idx];
            let id = field.id;
            let wire_type = match op_code {
                0 | 1 | 9 => 0, // Varint
                2 | 3 | 4 => 2, // LengthDelimited
                5 | 7 => 5,     // Fixed32
                6 | 8 => 1,     // Fixed64
                10 => 3,        // StartGroup
                _ => 2, // Default
            };
            
            let tag = (id << 3) | wire_type;
            write_varint32_fast(&mut buf, tag);

            match op_code {
                0 => { // Varint32 (Immediate)
                    // Handle zig-zag if needed? No, JS side should handle zig-zag or we do it here?
                    // JS side passes raw int32. If it's sint32, JS should have zig-zagged it?
                    // Or we check field type.
                    // For speed, let's assume JS passes raw value to write as varint.
                    // But wait, negative numbers?
                    // Int32Array is signed. Varint expects u64.
                    // If val_or_ref is negative, casting to u64 sign-extends.
                    // This is correct for int32/int64.
                    write_varint64_fast(&mut buf, val_or_ref as i64 as u64);
                },
                1 => { // Varint64 (Ref)
                    let val: JsUnknown = refs.get_element(val_or_ref as u32)?;
                    let v_type = val.get_type()?;
                    if v_type == ValueType::Number {
                        let num: f64 = val.coerce_to_number()?.get_double()?;
                        write_varint64_fast(&mut buf, num as u64);
                    } else if v_type == ValueType::Object {
                        let obj: JsObject = unsafe { val.cast() };
                        let low: i32 = obj.get_named_property("low")?;
                        let high: i32 = obj.get_named_property("high")?;
                        let v = ((high as i64) << 32) | (low as u32 as i64);
                        write_varint64_fast(&mut buf, v as u64);
                    } else {
                        write_varint64_fast(&mut buf, 0);
                    }
                },
                2 => { // String (Ref)
                    let val: JsString = refs.get_element(val_or_ref as u32)?;
                    let utf8 = val.into_utf8()?;
                    let bytes = utf8.as_slice();
                    write_varint32_fast(&mut buf, bytes.len() as u32);
                    buf.extend_from_slice(bytes);
                },
                3 => { // Bytes (Ref)
                    let val: JsUnknown = refs.get_element(val_or_ref as u32)?;
                    
                    let mut wrote = false;
                    if let Ok(buffer) = unsafe { val.cast::<JsBuffer>() }.into_value() {
                         let slice = buffer.as_ref();
                         write_varint32_fast(&mut buf, slice.len() as u32);
                         buf.extend_from_slice(slice);
                         wrote = true;
                    } else if val.is_typedarray()? {
                         let typed_array: JsTypedArray = unsafe { val.cast() };
                         if let Ok(data) = typed_array.into_value() {
                             let slice: &[u8] = data.as_ref();
                             write_varint32_fast(&mut buf, slice.len() as u32);
                             buf.extend_from_slice(slice);
                             wrote = true;
                         }
                    }
                    
                    if !wrote {
                         // Fallback: generic object with length (slow path)
                         let obj: JsObject = unsafe { val.cast() };
                         // Check if it has length property
                         if let Ok(len_val) = obj.get_named_property::<JsUnknown>("length") {
                             if let Ok(len_num) = len_val.coerce_to_number() {
                                 let len = len_num.get_uint32()?;
                                 write_varint32_fast(&mut buf, len);
                                 for i in 0..len {
                                     let item: JsUnknown = obj.get_element(i)?;
                                     // Assume it's a number
                                     let b = item.coerce_to_number()?.get_uint32()?;
                                     buf.push(b as u8);
                                 }
                                 wrote = true;
                             }
                         }
                    }

                    if !wrote {
                         write_varint32_fast(&mut buf, 0);
                    }
                },
                4 => { // Message (Ref)
                    let val: JsObject = refs.get_element(val_or_ref as u32)?;
                    // We need to encode this sub-message.
                    // We need its NativeType.
                    if let Some(nested) = &field.nested_type {
                        // Recursive call?
                        // We can't easily recurse `encode_hybrid` because we don't have the ops for the submessage!
                        // The JS side only generated ops for the TOP level message.
                        // Unless JS generated ops for the submessage too and put them in `ops`?
                        // But `ops` is flat.
                        // If JS generated ops for submessage, it would be inline?
                        // But we have a Ref to the object.
                        // This implies we need to traverse the object NOW.
                        // So we fall back to `encode_inner` (the old way) for submessages!
                        // This is the "Shallow Hybrid" approach.
                        let len_idx = buf.len();
                        write_varint32_fast(&mut buf, 0); // Placeholder for length
                        
                        nested.encode_inner(&env, &val, &mut buf)?;
                        
                        let msg_len = buf.len() - len_idx - 1;
                        // Fix length. This is tricky if length > 127.
                        // We need to move data.
                        // For PoC, let's just use `encode_inner` which handles length delimiter?
                        // No, `encode_inner` writes raw fields. It doesn't write length of the message itself.
                        // So we need to write length.
                        // To avoid moving data, we can compute size first?
                        // Or use a temporary buffer for submessage.
                        let mut sub_buf = Vec::new();
                        nested.encode_inner(&env, &val, &mut sub_buf)?;
                        
                        // Backtrack to overwrite length? No, we wrote 0 (1 byte).
                        // If sub_buf.len() > 127, we need more bytes.
                        // So we must pop the placeholder and write correct varint, then sub_buf.
                        buf.truncate(len_idx);
                        write_varint32_fast(&mut buf, sub_buf.len() as u32);
                        buf.extend_from_slice(&sub_buf);
                    }
                },
                5 => { // Fixed32 (Immediate)
                    let val = val_or_ref as u32;
                    buf.extend_from_slice(&val.to_le_bytes());
                },
                9 => { // Bool (Immediate)
                    write_varint64_fast(&mut buf, val_or_ref as u64);
                },
                6 => { // Fixed64 (Ref)
                    let val: JsUnknown = refs.get_element(val_or_ref as u32)?;
                    let v_type = val.get_type()?;
                    let v: u64 = if v_type == ValueType::BigInt {
                        let bigint = unsafe { val.cast::<napi::JsBigInt>() };
                        let (val, _lossless) = bigint.get_u64()?;
                        val
                    } else if v_type == ValueType::Number {
                        let num: f64 = val.coerce_to_number()?.get_double()?;
                        num as u64
                    } else if v_type == ValueType::String {
                        let js_str: napi::JsString = val.try_into()?;
                        let s = js_str.into_utf8()?.into_owned()?;
                        if let Ok(u) = s.parse::<u64>() {
                            u
                        } else if let Ok(i) = s.parse::<i64>() {
                            i as u64
                        } else {
                            0
                        }
                    } else if v_type == ValueType::Object {
                        let obj: JsObject = unsafe { val.cast() };
                        let low: i32 = obj.get_named_property("low")?;
                        let high: i32 = obj.get_named_property("high")?;
                        ((high as i64) << 32 | (low as u32 as i64)) as u64
                    } else {
                        0
                    };
                    buf.extend_from_slice(&v.to_le_bytes());
                },
                7 => { // Float (Ref)
                    let val: JsUnknown = refs.get_element(val_or_ref as u32)?;
                    let num: f64 = val.coerce_to_number()?.get_double()?;
                    buf.extend_from_slice(&(num as f32).to_le_bytes());
                },
                8 => { // Double (Ref)
                    let val: JsUnknown = refs.get_element(val_or_ref as u32)?;
                    let num: f64 = val.coerce_to_number()?.get_double()?;
                    buf.extend_from_slice(&num.to_le_bytes());
                },
                10 => { // Group (Ref)
                    let val: JsObject = refs.get_element(val_or_ref as u32)?;
                    if let Some(nested) = &field.nested_type {
                        nested.encode_inner(&env, &val, &mut buf)?;
                        // Write EndGroup Tag (WireType 4)
                        let end_tag = (id << 3) | 4;
                        write_varint32_fast(&mut buf, end_tag);
                    }
                },
                _ => {}
            }
        }
        
        env.create_buffer_with_data(buf).map(|b| b.into_raw())
    }

    #[napi]
    pub fn add_field(&mut self, name: String, id: u32, field_type: String, repeated: bool, required: bool, is_map: bool, key_type: Option<String>, nested: Option<&NativeType>, oneof_name: Option<String>) -> napi::Result<()> {
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
            oneof_name,
        });
        Ok(())
    }

    #[napi]
    pub fn encode(&self, env: Env, obj: JsObject) -> napi::Result<JsBuffer> {
        let mut pooled = BUFFER_POOL.acquire(1024);
        let buf = pooled.as_mut_vec();
        self.encode_inner(&env, &obj, buf)?;
        env.create_buffer_copy(buf).map(|b| b.into_raw())
    }

    fn encode_inner(&self, env: &Env, obj: &JsObject, buf: &mut Vec<u8>) -> napi::Result<()> {
        for field in &self.fields {
            if let Some(oneof_name) = &field.oneof_name {
                 let active_field_res: napi::Result<JsUnknown> = obj.get_named_property(oneof_name);
                 if let Ok(active_field_val) = active_field_res {
                     if active_field_val.get_type()? == ValueType::String {
                         let active_field_str: JsString = unsafe { active_field_val.cast() };
                         let s = active_field_str.into_utf8()?.into_owned()?;
                         if s != field.name {
                             continue;
                         }
                     } else {
                         continue;
                     }
                 } else {
                     continue;
                 }
            }

            // Schema-Driven: Directly access property by name
            // This avoids get_all_property_names() and temporary vector allocation
            let val_result: napi::Result<JsUnknown> = obj.get_named_property(&field.name);
            
            let val = match val_result {
                Ok(v) => v,
                Err(_) => {
                    // If property access fails (e.g. doesn't exist), treat as undefined
                    if field.required {
                        return Err(napi::Error::from_reason(format!("missing required '{}'", field.name)));
                    }
                    continue;
                }
            };

            let val_type = val.get_type()?;
            
            if val_type == ValueType::Undefined || val_type == ValueType::Null {
                if field.required {
                    return Err(napi::Error::from_reason(format!("missing required '{}'", field.name)));
                }
                continue;
            }

            // Check for default values if not required and not oneof (Proto3 semantics)
            if !field.required && field.oneof_name.is_none() {
                let is_default = match field.field_type.as_str() {
                    "bool" => {
                        if val_type == ValueType::Boolean {
                            let b_obj: JsBoolean = unsafe { val.cast() };
                            let b = b_obj.get_value()?;
                            !b
                        } else {
                            false
                        }
                    },
                    "string" => {
                        if val_type == ValueType::String {
                            let s: JsString = unsafe { val.cast() };
                            let utf8 = s.into_utf8()?;
                            utf8.as_slice().is_empty()
                        } else {
                            false
                        }
                    },
                    "int32" | "uint32" | "sint32" | "fixed32" | "sfixed32" | "enum" => {
                        if val_type == ValueType::Number {
                            let n: JsNumber = unsafe { val.cast() };
                            let v: f64 = n.get_double()?;
                            v == 0.0
                        } else {
                            false
                        }
                    },
                    "int64" | "uint64" | "sint64" | "fixed64" | "sfixed64" => {
                         if val_type == ValueType::Number {
                            let n: JsNumber = unsafe { val.cast() };
                            let v: f64 = n.get_double()?;
                            v == 0.0
                         } else if val_type == ValueType::BigInt {
                             let b: JsBigInt = unsafe { val.cast() };
                             let (v, _) = b.get_u64()?;
                             v == 0
                         } else if val_type == ValueType::Object {
                             // Long.js object { low: number, high: number, unsigned: boolean }
                             let obj: JsObject = unsafe { val.cast() };
                             let low_res: napi::Result<JsUnknown> = obj.get_named_property("low");
                             let high_res: napi::Result<JsUnknown> = obj.get_named_property("high");
                             
                             if let (Ok(low_val), Ok(high_val)) = (low_res, high_res) {
                                 let low: i32 = low_val.coerce_to_number()?.get_int32()?;
                                 let high: i32 = high_val.coerce_to_number()?.get_int32()?;
                                 low == 0 && high == 0
                             } else {
                                 false
                             }
                         } else {
                             false
                         }
                    },
                    "float" | "double" => {
                        if val_type == ValueType::Number {
                            let n: JsNumber = unsafe { val.cast() };
                            let v: f64 = n.get_double()?;
                            v == 0.0
                        } else {
                            false
                        }
                    },
                    "bytes" => {
                        if val.is_buffer()? {
                             let b: JsBuffer = unsafe { val.cast() };
                             let v = b.into_value()?;
                             v.len() == 0
                        } else {
                            false
                        }
                    },
                    _ => false
                };

                if is_default {
                    continue;
                }
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
                self.encode_value(env, buf, field, val)?;
            }
        }
        Ok(())
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
            oneof_name: None,
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
            oneof_name: None,
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
