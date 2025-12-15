use napi::bindgen_prelude::*;
use napi::{JsBuffer, JsObject, JsString, JsBigInt, JsUnknown, Env, ValueType, NapiRaw};
use napi_derive::napi;
use rustc_hash::FxHashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use napi::NapiValue;

// 引入我们之前写好的快速读写原语
use crate::writer::{write_varint32_fast, write_varint64_fast};
use std::convert::TryInto;

struct Decoder<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Decoder<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    fn eof(&self) -> bool {
        self.pos >= self.buf.len()
    }


    #[inline(always)]
    fn read_varint(&mut self) -> napi::Result<u64> {
        let mut value: u64 = 0;
        let mut shift = 0;
        while self.pos < self.buf.len() {
            let b = self.buf[self.pos];
            self.pos += 1;
            value |= ((b & 0x7F) as u64) << shift;
            if (b & 0x80) == 0 {
                return Ok(value);
            }
            shift += 7;
            if shift > 63 {
                return Err(napi::Error::from_reason("Varint too large".to_string()));
            }
        }
        Err(napi::Error::from_reason("Unexpected EOF".to_string()))
    }

    #[inline(always)]
    fn read_u32(&mut self) -> napi::Result<u32> {
        if self.pos + 4 > self.buf.len() {
            return Err(napi::Error::from_reason("Unexpected EOF".to_string()));
        }
        let bytes = &self.buf[self.pos..self.pos+4];
        self.pos += 4;
        Ok(u32::from_le_bytes(bytes.try_into().unwrap()))
    }

    #[inline(always)]
    fn read_u64(&mut self) -> napi::Result<u64> {
        if self.pos + 8 > self.buf.len() {
            return Err(napi::Error::from_reason("Unexpected EOF".to_string()));
        }
        let bytes = &self.buf[self.pos..self.pos+8];
        self.pos += 8;
        Ok(u64::from_le_bytes(bytes.try_into().unwrap()))
    }

    fn read_bytes(&mut self, len: usize) -> napi::Result<Vec<u8>> {
        if self.pos + len > self.buf.len() {
            return Err(napi::Error::from_reason("Unexpected EOF".to_string()));
        }
        let bytes = self.buf[self.pos..self.pos+len].to_vec();
        self.pos += len;
        Ok(bytes)
    }

    fn read_bytes_ref(&mut self, len: usize) -> napi::Result<(u32, u32)> {
        if self.pos + len > self.buf.len() {
            return Err(napi::Error::from_reason("Unexpected EOF".to_string()));
        }
        let offset = self.pos as u32;
        self.pos += len;
        Ok((offset, len as u32))
    }

    fn skip_field(&mut self, wire_type: u32) -> napi::Result<()> {
        match wire_type {
            0 => { self.read_varint()?; },
            1 => { self.read_u64()?; },
            5 => { self.read_u32()?; },
            2 => {
                let len = self.read_varint()? as usize;
                if self.pos + len > self.buf.len() {
                    return Err(napi::Error::from_reason("Unexpected EOF".to_string()));
                }
                self.pos += len;
            },
            _ => return Err(napi::Error::from_reason(format!("Unknown wire type {}", wire_type))),
        }
        Ok(())
    }
}

#[inline(always)]
fn varint_size(v: u64) -> usize {
    if v < (1 << 7) { return 1; }
    if v < (1 << 14) { return 2; }
    if v < (1 << 21) { return 3; }
    if v < (1 << 28) { return 4; }
    if v < (1 << 35) { return 5; }
    if v < (1 << 42) { return 6; }
    if v < (1 << 49) { return 7; }
    if v < (1 << 56) { return 8; }
    if v < (1 << 63) { return 9; }
    10
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MapKey {
    Varint(u64),
    Bit32(u32),
    Bit64(u64),
    String(String),
    Bool(bool),
}

#[derive(Clone, Debug)]
pub enum Value {
    Varint(u64),      // int32, int64, uint32, bool, enum
    Bit64(u64),       // fixed64, sfixed64, double
    Bit32(u32),       // fixed32, sfixed32, float
    String(String),   // string
    StringRef(u32, u32), // string ref (offset, len)
    Bytes(Vec<u8>),   // bytes
    BytesRef(u32, u32), // bytes ref (offset, len)
    Nested(Box<ManagedMessage>), // nested message
    NestedRef(u32, u32), // nested message ref (offset, len)
    Group(Box<ManagedMessage>), // group message
    Repeated(Vec<Value>), // repeated fields (non-packed or complex types)
    PackedVarint(Vec<u64>), // packed varint (int32, int64, bool, etc.)
    PackedBit32(Vec<u32>),  // packed fixed32, float
    PackedBit64(Vec<u64>),  // packed fixed64, double
    
    // Optimized Non-Packed Repeated Fields
    RepeatedVarint(Vec<u64>),
    RepeatedBit32(Vec<u32>),
    RepeatedBit64(Vec<u64>),

    Map(Box<FxHashMap<MapKey, Value>>), // map fields - Boxed to reduce enum size
}

#[napi]
#[derive(Debug)]
pub struct ManagedMessage {
    // Map Field ID -> Value
    fields: FxHashMap<u32, Value>,
    cached_size: AtomicUsize,
    buffer: Option<Arc<Vec<u8>>>,
}

#[napi]
impl ManagedMessage {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            fields: FxHashMap::default(),
            cached_size: AtomicUsize::new(usize::MAX),
            buffer: None,
        }
    }

    fn invalidate_cache(&self) {
        self.cached_size.store(usize::MAX, Ordering::Relaxed);
    }

    #[napi]
    pub fn clear(&mut self) {
        self.fields.clear();
        self.invalidate_cache();
    }

    #[napi]
    pub fn remove(&mut self, id: u32) {
        self.fields.remove(&id);
        self.invalidate_cache();
    }

    #[napi]
    pub fn has(&self, id: u32) -> bool {
        self.fields.contains_key(&id)
    }

    #[napi(factory)]
    pub fn decode(buffer: JsBuffer) -> napi::Result<Self> {
        let bytes = buffer.into_value()?.to_vec();
        Self::decode_from_vec(bytes)
    }

    fn decode_from_vec(bytes: Vec<u8>) -> napi::Result<Self> {
        let arc_buf = Arc::new(bytes);
        let mut decoder = Decoder::new(&arc_buf);
        let mut msg = Self::decode_with_decoder(&mut decoder, 0)?;
        msg.buffer = Some(arc_buf);
        Ok(msg)
    }

    fn decode_from_bytes(bytes: &[u8]) -> napi::Result<Self> {
        Self::decode_from_vec(bytes.to_vec())
    }

    fn decode_with_decoder(decoder: &mut Decoder, end_tag: u32) -> napi::Result<Self> {
        let mut msg = ManagedMessage::new();
        
        while !decoder.eof() {
            let tag = decoder.read_varint()? as u32;
            if tag == 0 { break; }
            
            if tag == end_tag {
                return Ok(msg);
            }

            let field_id = tag >> 3;
            let wire_type = tag & 7;
            
            if wire_type == 4 {
                 return Err(napi::Error::from_reason(format!("Unexpected EndGroup tag {}", tag)));
            }

            let value = match wire_type {
                0 => Value::Varint(decoder.read_varint()?),
                1 => Value::Bit64(decoder.read_u64()?),
                5 => Value::Bit32(decoder.read_u32()?),
                2 => {
                    let len = decoder.read_varint()? as usize;
                    let (off, l) = decoder.read_bytes_ref(len)?;
                    Value::BytesRef(off, l)
                },
                3 => {
                    let group_end_tag = (field_id << 3) | 4;
                    let group_msg = Self::decode_with_decoder(decoder, group_end_tag)?;
                    Value::Group(Box::new(group_msg))
                },
                _ => return Err(napi::Error::from_reason(format!("Unknown wire type {}", wire_type))),
            };
            
            msg.add_value(field_id, value);
        }
        
        if end_tag != 0 {
             return Err(napi::Error::from_reason("Unexpected EOF inside group".to_string()));
        }
        
        Ok(msg)
    }

    fn add_value(&mut self, id: u32, val: Value) {
        use std::collections::hash_map::Entry;
        match self.fields.entry(id) {
            Entry::Occupied(mut entry) => {
                let existing = entry.get_mut();
                match (existing, val) {
                    // Optimized RepeatedVarint
                    (Value::RepeatedVarint(vec), Value::Varint(v)) => vec.push(v),
                    (Value::Varint(old), Value::Varint(new)) => {
                        *entry.get_mut() = Value::RepeatedVarint(vec![*old, new]);
                    },
                    
                    // Optimized RepeatedBit32
                    (Value::RepeatedBit32(vec), Value::Bit32(v)) => vec.push(v),
                    (Value::Bit32(old), Value::Bit32(new)) => {
                        *entry.get_mut() = Value::RepeatedBit32(vec![*old, new]);
                    },

                    // Optimized RepeatedBit64
                    (Value::RepeatedBit64(vec), Value::Bit64(v)) => vec.push(v),
                    (Value::Bit64(old), Value::Bit64(new)) => {
                        *entry.get_mut() = Value::RepeatedBit64(vec![*old, new]);
                    },

                    // Fallback to generic Repeated
                    (Value::Repeated(vec), val) => vec.push(val),
                    (old_val, new_val) => {
                        let old = std::mem::replace(old_val, Value::Varint(0));
                        *old_val = Value::Repeated(vec![old, new_val]);
                    }
                }
            },
            Entry::Vacant(entry) => {
                entry.insert(val);
            }
        }
        self.invalidate_cache();
    }

    // --- Setters (By ID) ---
    // 为了性能，我们直接使用 ID，不处理 Name。Name -> ID 的映射由 JS 侧的反射层处理。

    #[napi]
    pub fn set_int32(&mut self, id: u32, value: i32) {
        self.fields.insert(id, Value::Varint(value as i64 as u64));
        self.invalidate_cache();
    }

    #[napi]
    pub fn set_uint32(&mut self, id: u32, value: u32) {
        self.fields.insert(id, Value::Varint(value as u64));
        self.invalidate_cache();
    }

    #[napi]
    pub fn set_float(&mut self, id: u32, value: f64) {
        // Proto float is 32-bit, but we store as Bit32 representation
        let v = value as f32;
        self.fields.insert(id, Value::Bit32(u32::from_le_bytes(v.to_le_bytes())));
        self.invalidate_cache();
    }

    #[napi]
    pub fn set_double(&mut self, id: u32, value: f64) {
        self.fields.insert(id, Value::Bit64(u64::from_le_bytes(value.to_le_bytes())));
        self.invalidate_cache();
    }

    #[napi]
    pub fn set_bool(&mut self, id: u32, value: bool) {
        self.fields.insert(id, Value::Varint(if value { 1 } else { 0 }));
        self.invalidate_cache();
    }

    #[napi]
    pub fn set_int64(&mut self, id: u32, value: BigInt) {
        self.fields.insert(id, Value::Varint(value.get_i64().0 as u64));
        self.invalidate_cache();
    }

    #[napi]
    pub fn set_uint64(&mut self, id: u32, value: BigInt) {
        self.fields.insert(id, Value::Varint(value.get_u64().1));
        self.invalidate_cache();
    }

    #[napi]
    pub fn set_sint32(&mut self, id: u32, value: i32) {
        let encoded = ((value << 1) ^ (value >> 31)) as u32;
        self.fields.insert(id, Value::Varint(encoded as u64));
        self.invalidate_cache();
    }

    #[napi]
    pub fn set_sint64(&mut self, id: u32, value: BigInt) {
        let v = value.get_i64().0;
        let encoded = ((v << 1) ^ (v >> 63)) as u64;
        self.fields.insert(id, Value::Varint(encoded));
        self.invalidate_cache();
    }

    #[napi]
    pub fn set_sfixed64(&mut self, id: u32, value: BigInt) {
        self.fields.insert(id, Value::Bit64(value.get_i64().0 as u64));
        self.invalidate_cache();
    }

    #[napi]
    pub fn set_fixed64(&mut self, id: u32, value: BigInt) {
        self.fields.insert(id, Value::Bit64(value.get_u64().1));
        self.invalidate_cache();
    }

    #[napi]
    pub fn set_map(&mut self, env: Env, id: u32, obj: JsObject, key_type: u32, value_type: u32) -> napi::Result<()> {
        let mut map = FxHashMap::default();
        let keys = obj.get_property_names()?;
        let len = keys.get_array_length()?;
        
        for i in 0..len {
            let key_str: JsString = keys.get_element(i)?;
            let key_utf8 = key_str.into_utf8()?.into_owned()?;
            
            let val_js: JsUnknown = obj.get_property(key_str)?;
            
            let map_key = parse_map_key(&key_utf8, key_type)?;
            let map_val = parse_map_value(&env, val_js, value_type)?;
            
            map.insert(map_key, map_val);
        }
        
        self.fields.insert(id, Value::Map(Box::new(map)));
        self.invalidate_cache();
        Ok(())
    }

    #[napi]
    pub fn set_string(&mut self, id: u32, value: String) {
        self.fields.insert(id, Value::String(value));
        self.invalidate_cache();
    }

    #[napi]
    pub fn set_bytes(&mut self, env: Env, id: u32, value: JsBuffer) -> napi::Result<()> {
        let obj = unsafe { JsObject::from_raw_unchecked(env.raw(), value.raw()) };
        let len: u32 = obj.get_named_property("length")?;
        let vec = if len == 0 {
            Vec::new()
        } else {
            value.into_value()?.to_vec()
        };
        self.fields.insert(id, Value::Bytes(vec));
        self.invalidate_cache();
        Ok(())
    }

    #[napi]
    pub fn set_nested(&mut self, id: u32, msg: &ManagedMessage) {
        // 注意：这里我们 Clone 了整个消息。
        // 在生产级实现中，可能需要更复杂的引用管理或 Arc/Rc，但为了演示性能，Clone 是安全的。
        self.fields.insert(id, Value::Nested(Box::new(msg.clone())));
        self.invalidate_cache();
    }

    #[napi]
    pub fn set_group(&mut self, id: u32, msg: &ManagedMessage) {
        self.fields.insert(id, Value::Group(Box::new(msg.clone())));
        self.invalidate_cache();
    }

    // --- Repeated Setters (Adders) ---

    fn get_or_init_repeated(&mut self, id: u32) -> &mut Vec<Value> {
        let val = self.fields.entry(id)
            .or_insert_with(|| Value::Repeated(Vec::new()));
        
        match val {
            Value::Repeated(vec) => vec,
            _ => panic!("Field {} is not repeated", id),
        }
    }

    #[napi]
    pub fn add_int32(&mut self, id: u32, value: i32) {
        let vec = self.get_or_init_repeated(id);
        vec.push(Value::Varint(value as i64 as u64));
    }

    #[napi]
    pub fn add_string(&mut self, id: u32, value: String) {
        let vec = self.get_or_init_repeated(id);
        vec.push(Value::String(value));
    }

    #[napi]
    pub fn add_nested(&mut self, id: u32, msg: &ManagedMessage) {
        let vec = self.get_or_init_repeated(id);
        vec.push(Value::Nested(Box::new(msg.clone())));
    }

    #[napi]
    pub fn add_group(&mut self, id: u32, msg: &ManagedMessage) {
        let vec = self.get_or_init_repeated(id);
        vec.push(Value::Group(Box::new(msg.clone())));
    }

    #[napi]
    pub fn add_uint32(&mut self, id: u32, value: u32) {
        let vec = self.get_or_init_repeated(id);
        vec.push(Value::Varint(value as u64));
    }

    #[napi]
    pub fn add_int64(&mut self, id: u32, value: BigInt) {
        let vec = self.get_or_init_repeated(id);
        vec.push(Value::Varint(value.get_i64().0 as u64));
    }

    #[napi]
    pub fn add_uint64(&mut self, id: u32, value: BigInt) {
        let vec = self.get_or_init_repeated(id);
        vec.push(Value::Varint(value.get_u64().1));
    }

    #[napi]
    pub fn add_sint32(&mut self, id: u32, value: i32) {
        let vec = self.get_or_init_repeated(id);
        let encoded = ((value << 1) ^ (value >> 31)) as u32;
        vec.push(Value::Varint(encoded as u64));
    }

    #[napi]
    pub fn add_sint64(&mut self, id: u32, value: BigInt) {
        let vec = self.get_or_init_repeated(id);
        let v = value.get_i64().0;
        let encoded = ((v << 1) ^ (v >> 63)) as u64;
        vec.push(Value::Varint(encoded));
    }

    #[napi]
    pub fn add_fixed64(&mut self, id: u32, value: BigInt) {
        let vec = self.get_or_init_repeated(id);
        vec.push(Value::Bit64(value.get_u64().1));
    }

    #[napi]
    pub fn add_sfixed64(&mut self, id: u32, value: BigInt) {
        let vec = self.get_or_init_repeated(id);
        vec.push(Value::Bit64(value.get_i64().0 as u64));
    }

    #[napi]
    pub fn add_bool(&mut self, id: u32, value: bool) {
        let vec = self.get_or_init_repeated(id);
        vec.push(Value::Varint(if value { 1 } else { 0 }));
    }

    #[napi]
    pub fn add_float(&mut self, id: u32, value: f64) {
        let vec = self.get_or_init_repeated(id);
        let v = value as f32;
        vec.push(Value::Bit32(u32::from_le_bytes(v.to_le_bytes())));
    }

    #[napi]
    pub fn add_double(&mut self, id: u32, value: f64) {
        let vec = self.get_or_init_repeated(id);
        vec.push(Value::Bit64(u64::from_le_bytes(value.to_le_bytes())));
    }

    #[napi]
    pub fn add_bytes(&mut self, env: Env, id: u32, value: JsBuffer) -> napi::Result<()> {
        let vec = self.get_or_init_repeated(id);
        let obj = unsafe { JsObject::from_raw_unchecked(env.raw(), value.raw()) };
        let len: u32 = obj.get_named_property("length")?;
        let bytes = if len == 0 {
            Vec::new()
        } else {
            value.into_value()?.to_vec()
        };
        vec.push(Value::Bytes(bytes));
        Ok(())
    }


    // --- Encoding ---

    #[napi]
    pub fn encode(&self, env: Env) -> napi::Result<JsBuffer> {
        let mut buf = Vec::with_capacity(1024);
        self.encode_inner(&mut buf);
        env.create_buffer_with_data(buf).map(|b| b.into_raw())
    }

    fn encode_inner(&self, buf: &mut Vec<u8>) {
        // 简单的遍历编码。
        // 注意：HashMap 迭代顺序是不确定的。Proto 标准允许乱序，但通常建议按 ID 排序。
        // 为了极致性能，我们先不排序（或者在生产中收集 keys 排序）。
        // 这里为了确定性，我们先收集 keys。
        let mut keys: Vec<&u32> = self.fields.keys().collect();
        keys.sort_unstable();

        for id in keys {
            let val = &self.fields[id];
            self.encode_field(buf, *id, val);
        }
    }

    fn encode_field(&self, buf: &mut Vec<u8>, id: u32, val: &Value) {
        match val {
            Value::Varint(v) => {
                // WireType 0
                let tag = (id << 3) | 0;
                write_varint32_fast(buf, tag);
                write_varint64_fast(buf, *v);
            },
            Value::Bit64(v) => {
                // WireType 1
                let tag = (id << 3) | 1;
                write_varint32_fast(buf, tag);
                buf.extend_from_slice(&v.to_le_bytes());
            },
            Value::Bit32(v) => {
                // WireType 5
                let tag = (id << 3) | 5;
                write_varint32_fast(buf, tag);
                buf.extend_from_slice(&v.to_le_bytes());
            },
            Value::String(s) => {
                // WireType 2
                let tag = (id << 3) | 2;
                write_varint32_fast(buf, tag);
                let bytes = s.as_bytes();
                write_varint32_fast(buf, bytes.len() as u32);
                buf.extend_from_slice(bytes);
            },
            Value::Bytes(b) => {
                // WireType 2
                let tag = (id << 3) | 2;
                write_varint32_fast(buf, tag);
                write_varint32_fast(buf, b.len() as u32);
                buf.extend_from_slice(b);
            },
            Value::StringRef(off, len) | Value::BytesRef(off, len) | Value::NestedRef(off, len) => {
                let tag = (id << 3) | 2;
                write_varint32_fast(buf, tag);
                write_varint32_fast(buf, *len);
                if let Some(buffer) = &self.buffer {
                    let start = *off as usize;
                    let end = start + *len as usize;
                    if end <= buffer.len() {
                        buf.extend_from_slice(&buffer[start..end]);
                    }
                }
            },
            Value::Nested(msg) => {
                // WireType 2
                let tag = (id << 3) | 2;
                write_varint32_fast(buf, tag);
                
                let size = msg.compute_size_inner();
                write_varint32_fast(buf, size as u32);
                msg.encode_inner(buf);
            },
            Value::Group(msg) => {
                // WireType 3 (StartGroup)
                let tag = (id << 3) | 3;
                write_varint32_fast(buf, tag);
                
                msg.encode_inner(buf);
                
                // WireType 4 (EndGroup)
                let end_tag = (id << 3) | 4;
                write_varint32_fast(buf, end_tag);
            },
            Value::Repeated(vec) => {
                // 简化处理：假设非 packed
                for item in vec {
                    self.encode_field(buf, id, item);
                }
            },
            Value::PackedVarint(vec) => {
                let tag = (id << 3) | 2;
                write_varint32_fast(buf, tag);
                
                let mut len = 0;
                for v in vec {
                    len += varint_size(*v) as u32;
                }
                write_varint32_fast(buf, len);
                for v in vec {
                    write_varint64_fast(buf, *v);
                }
            },
            Value::PackedBit32(vec) => {
                let tag = (id << 3) | 2;
                write_varint32_fast(buf, tag);
                let len = vec.len() * 4;
                write_varint32_fast(buf, len as u32);
                
                #[cfg(target_endian = "little")]
                {
                    let ptr = vec.as_ptr() as *const u8;
                    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
                    buf.extend_from_slice(slice);
                }
                #[cfg(not(target_endian = "little"))]
                {
                    for v in vec {
                        buf.extend_from_slice(&v.to_le_bytes());
                    }
                }
            },
            Value::PackedBit64(vec) => {
                let tag = (id << 3) | 2;
                write_varint32_fast(buf, tag);
                let len = vec.len() * 8;
                write_varint32_fast(buf, len as u32);
                
                #[cfg(target_endian = "little")]
                {
                    let ptr = vec.as_ptr() as *const u8;
                    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
                    buf.extend_from_slice(slice);
                }
                #[cfg(not(target_endian = "little"))]
                {
                    for v in vec {
                        buf.extend_from_slice(&v.to_le_bytes());
                    }
                }
            },
            Value::RepeatedVarint(vec) => {
                // WireType 0
                let tag = (id << 3) | 0;
                for v in vec {
                    write_varint32_fast(buf, tag);
                    write_varint64_fast(buf, *v);
                }
            },
            Value::RepeatedBit32(vec) => {
                // WireType 5
                let tag = (id << 3) | 5;
                for v in vec {
                    write_varint32_fast(buf, tag);
                    buf.extend_from_slice(&v.to_le_bytes());
                }
            },
            Value::RepeatedBit64(vec) => {
                // WireType 1
                let tag = (id << 3) | 1;
                for v in vec {
                    write_varint32_fast(buf, tag);
                    buf.extend_from_slice(&v.to_le_bytes());
                }
            },
            Value::Map(map) => {
                for (k, v) in map.iter() {
                    let tag = (id << 3) | 2;
                    write_varint32_fast(buf, tag);
                    
                    let key_size = compute_map_key_size(1, k);
                    let val_size = self.compute_field_size(2, v);
                    let entry_size = key_size + val_size;
                    
                    write_varint32_fast(buf, entry_size as u32);
                    
                    encode_map_key(buf, 1, k);
                    self.encode_field(buf, 2, v);
                }
            }
        }
    }

    fn compute_size_inner(&self) -> usize {
        let cached = self.cached_size.load(Ordering::Relaxed);
        if cached != usize::MAX {
            return cached;
        }

        let mut size = 0;
        for (id, val) in &self.fields {
            size += self.compute_field_size(*id, val);
        }
        
        self.cached_size.store(size, Ordering::Relaxed);
        size
    }

    fn compute_field_size(&self, id: u32, val: &Value) -> usize {
        match val {
            Value::Varint(v) => {
                let tag = (id << 3) | 0;
                varint_size(tag as u64) + varint_size(*v)
            },
            Value::Bit64(_) => {
                let tag = (id << 3) | 1;
                varint_size(tag as u64) + 8
            },
            Value::Bit32(_) => {
                let tag = (id << 3) | 5;
                varint_size(tag as u64) + 4
            },
            Value::String(s) => {
                let tag = (id << 3) | 2;
                let len = s.len();
                varint_size(tag as u64) + varint_size(len as u64) + len
            },
            Value::Bytes(b) => {
                let tag = (id << 3) | 2;
                let len = b.len();
                varint_size(tag as u64) + varint_size(len as u64) + len
            },
            Value::StringRef(_, len) | Value::BytesRef(_, len) | Value::NestedRef(_, len) => {
                let tag = (id << 3) | 2;
                varint_size(tag as u64) + varint_size(*len as u64) + (*len as usize)
            },
            Value::Nested(msg) => {
                let tag = (id << 3) | 2;
                let len = msg.compute_size_inner();
                varint_size(tag as u64) + varint_size(len as u64) + len
            },
            Value::Group(msg) => {
                let tag = (id << 3) | 3;
                let end_tag = (id << 3) | 4;
                varint_size(tag as u64) + msg.compute_size_inner() + varint_size(end_tag as u64)
            },
            Value::Repeated(vec) => {
                let mut size = 0;
                for item in vec {
                    size += self.compute_field_size(id, item);
                }
                size
            },
            Value::PackedVarint(vec) => {
                let tag = (id << 3) | 2;
                let mut data_len = 0;
                for v in vec {
                    data_len += varint_size(*v);
                }
                varint_size(tag as u64) + varint_size(data_len as u64) + data_len
            },
            Value::PackedBit32(vec) => {
                let tag = (id << 3) | 2;
                let data_len = vec.len() * 4;
                varint_size(tag as u64) + varint_size(data_len as u64) + data_len
            },
            Value::PackedBit64(vec) => {
                let tag = (id << 3) | 2;
                let data_len = vec.len() * 8;
                varint_size(tag as u64) + varint_size(data_len as u64) + data_len
            },
            Value::RepeatedVarint(vec) => {
                let tag = (id << 3) | 0;
                let tag_size = varint_size(tag as u64);
                let mut size = 0;
                for v in vec {
                    size += tag_size + varint_size(*v);
                }
                size
            },
            Value::RepeatedBit32(vec) => {
                let tag = (id << 3) | 5;
                let tag_size = varint_size(tag as u64);
                vec.len() * (tag_size + 4)
            },
            Value::RepeatedBit64(vec) => {
                let tag = (id << 3) | 1;
                let tag_size = varint_size(tag as u64);
                vec.len() * (tag_size + 8)
            },
            Value::Map(map) => {
                let mut total = 0;
                for (k, v) in map.iter() {
                    let tag = (id << 3) | 2;
                    let key_size = compute_map_key_size(1, k);
                    let val_size = self.compute_field_size(2, v);
                    let entry_size = key_size + val_size;
                    total += varint_size(tag as u64) + varint_size(entry_size as u64) + entry_size;
                }
                total
            }
        }
    }

    // --- Getters ---

    #[napi]
    pub fn get_int32(&self, id: u32) -> Option<i32> {
        self.fields.get(&id).and_then(|v| match v {
            Value::Varint(i) => Some(*i as i32),
            Value::Bit32(i) => Some(*i as i32),
            Value::Bit64(i) => Some(*i as i32),
            _ => None,
        })
    }

    #[napi]
    pub fn get_uint32(&self, id: u32) -> Option<u32> {
        self.fields.get(&id).and_then(|v| match v {
            Value::Varint(i) => Some(*i as u32),
            Value::Bit32(i) => Some(*i),
            Value::Bit64(i) => Some(*i as u32),
            _ => None,
        })
    }

    #[napi]
    pub fn get_float(&self, id: u32) -> Option<f64> {
        self.fields.get(&id).and_then(|v| match v {
            Value::Bit32(i) => Some(f32::from_bits(*i) as f64),
            Value::Bit64(i) => Some(f64::from_bits(*i)),
            _ => None,
        })
    }

    #[napi]
    pub fn get_double(&self, id: u32) -> Option<f64> {
        self.fields.get(&id).and_then(|v| match v {
            Value::Bit64(i) => Some(f64::from_bits(*i)),
            Value::Bit32(i) => Some(f32::from_bits(*i) as f64),
            _ => None,
        })
    }

    #[napi]
    pub fn get_int64(&self, env: Env, id: u32) -> Option<JsBigInt> {
        self.fields.get(&id).and_then(|v| match v {
            Value::Varint(i) => env.create_bigint_from_i64(*i as i64).ok(),
            Value::Bit64(i) => env.create_bigint_from_i64(*i as i64).ok(),
            Value::Bit32(i) => env.create_bigint_from_i64(*i as i64).ok(),
            _ => None,
        })
    }

    #[napi]
    pub fn get_uint64(&self, env: Env, id: u32) -> Option<JsBigInt> {
        self.fields.get(&id).and_then(|v| match v {
            Value::Varint(i) => env.create_bigint_from_u64(*i).ok(),
            Value::Bit64(i) => env.create_bigint_from_u64(*i).ok(),
            Value::Bit32(i) => env.create_bigint_from_u64(*i as u64).ok(),
            _ => None,
        })
    }

    #[napi]
    pub fn get_sint64(&self, env: Env, id: u32) -> Option<JsBigInt> {
        self.fields.get(&id).and_then(|v| match v {
            Value::Varint(i) => {
                let n = *i;
                let decoded = (n >> 1) as i64 ^ -((n & 1) as i64);
                env.create_bigint_from_i64(decoded).ok()
            },
            _ => None,
        })
    }

    #[napi]
    pub fn get_sfixed64(&self, env: Env, id: u32) -> Option<JsBigInt> {
        self.fields.get(&id).and_then(|v| match v {
            Value::Bit64(i) => env.create_bigint_from_i64(*i as i64).ok(),
            _ => None,
        })
    }

    #[napi]
    pub fn get_bool(&self, id: u32) -> Option<bool> {
        self.fields.get(&id).and_then(|v| match v {
            Value::Varint(i) => Some(*i != 0),
            _ => None,
        })
    }

    #[napi]
    pub fn get_string(&self, id: u32) -> Option<String> {
        self.fields.get(&id).and_then(|v| match v {
            Value::String(s) => Some(s.clone()),
            Value::Bytes(b) => String::from_utf8(b.clone()).ok(),
            Value::StringRef(off, len) | Value::BytesRef(off, len) => {
                if let Some(buf) = &self.buffer {
                    let start = *off as usize;
                    let end = start + *len as usize;
                    if end <= buf.len() {
                        std::str::from_utf8(&buf[start..end]).ok().map(|s| s.to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            },
            _ => None,
        })
    }

    #[napi]
    pub fn get_bytes(&self, env: Env, id: u32) -> Option<JsBuffer> {
        self.fields.get(&id).and_then(|v| match v {
            Value::Bytes(b) => env.create_buffer_with_data(b.clone()).ok().map(|b| b.into_raw()),
            Value::BytesRef(off, len) | Value::StringRef(off, len) => {
                 if let Some(buf) = &self.buffer {
                    let start = *off as usize;
                    let end = start + *len as usize;
                    if end <= buf.len() {
                        env.create_buffer_with_data(buf[start..end].to_vec()).ok().map(|b| b.into_raw())
                    } else {
                        None
                    }
                 } else {
                     None
                 }
            },
            _ => None,
        })
    }

    #[napi]
    pub fn get_nested(&self, id: u32) -> Option<ManagedMessage> {
        self.fields.get(&id).and_then(|v| match v {
            Value::Nested(msg) => Some(*msg.clone()),
            Value::Group(msg) => Some(*msg.clone()),
            Value::Bytes(b) => {
                ManagedMessage::decode_from_bytes(b).ok()
            },
            Value::BytesRef(off, len) => {
                 if let Some(buf) = &self.buffer {
                    let start = *off as usize;
                    let end = start + *len as usize;
                    if end <= buf.len() {
                        ManagedMessage::decode_from_bytes(&buf[start..end]).ok()
                    } else {
                        None
                    }
                 } else {
                     None
                 }
            },
            _ => None,
        })
    }

    #[napi]
    pub fn get_map(&self, env: Env, id: u32, key_type: u32, value_type: u32) -> Option<JsObject> {
        self.fields.get(&id).and_then(|v| {
            let mut obj = env.create_object().ok()?;
            match v {
                Value::Map(map) => {
                    for (key, val) in map.iter() {
                        let key_str = map_key_to_string(key);
                        let js_val = value_to_js(&env, val).ok()?;
                        obj.set_named_property(&key_str, js_val).ok()?;
                    }
                },
                Value::Bytes(b) => {
                    let (k, v) = parse_map_entry_from_bytes(b, key_type, value_type).ok()?;
                    let key_str = map_key_to_string(&k);
                    let js_val = value_to_js(&env, &v).ok()?;
                    obj.set_named_property(&key_str, js_val).ok()?;
                },
                Value::BytesRef(off, len) => {
                     if let Some(buf) = &self.buffer {
                        let start = *off as usize;
                        let end = start + *len as usize;
                        if end <= buf.len() {
                            let (k, v) = parse_map_entry_from_bytes(&buf[start..end], key_type, value_type).ok()?;
                            let key_str = map_key_to_string(&k);
                            let js_val = value_to_js(&env, &v).ok()?;
                            obj.set_named_property(&key_str, js_val).ok()?;
                        }
                     }
                },
                Value::Repeated(vec) => {
                    for item in vec {
                        if let Value::Bytes(b) = item {
                            if let Ok((k, v)) = parse_map_entry_from_bytes(b, key_type, value_type) {
                                let key_str = map_key_to_string(&k);
                                if let Ok(js_val) = value_to_js(&env, &v) {
                                    obj.set_named_property(&key_str, js_val).ok()?;
                                }
                            }
                        } else if let Value::BytesRef(off, len) = item {
                             if let Some(buf) = &self.buffer {
                                let start = *off as usize;
                                let end = start + *len as usize;
                                if end <= buf.len() {
                                    if let Ok((k, v)) = parse_map_entry_from_bytes(&buf[start..end], key_type, value_type) {
                                        let key_str = map_key_to_string(&k);
                                        if let Ok(js_val) = value_to_js(&env, &v) {
                                            obj.set_named_property(&key_str, js_val).ok()?;
                                        }
                                    }
                                }
                             }
                        }
                    }
                },
                _ => return None,
            }
            Some(obj)
        })
    }

    #[napi]
    pub fn get_int32_array(&self, id: u32) -> Option<Vec<i32>> {
        self.fields.get(&id).and_then(|v| match v {
            Value::Repeated(vec) => Some(vec.iter().filter_map(|x| if let Value::Varint(i) = x { Some(*i as i32) } else { None }).collect()),
            Value::RepeatedVarint(vec) => Some(vec.iter().map(|x| *x as i32).collect()),
            Value::PackedVarint(vec) => Some(vec.iter().map(|x| *x as i32).collect()),
            Value::Bytes(b) => parse_packed_varints(b).ok().map(|vec| vec.iter().map(|x| *x as i32).collect()),
            Value::BytesRef(off, len) => {
                 if let Some(buf) = &self.buffer {
                    let start = *off as usize;
                    let end = start + *len as usize;
                    if end <= buf.len() {
                        parse_packed_varints(&buf[start..end]).ok().map(|vec| vec.iter().map(|x| *x as i32).collect())
                    } else { None }
                 } else { None }
            },
            _ => None,
        })
    }

    #[napi]
    pub fn get_uint32_array(&self, id: u32) -> Option<Vec<u32>> {
        self.fields.get(&id).and_then(|v| match v {
            Value::Repeated(vec) => Some(vec.iter().filter_map(|x| if let Value::Varint(i) = x { Some(*i as u32) } else { None }).collect()),
            Value::RepeatedVarint(vec) => Some(vec.iter().map(|x| *x as u32).collect()),
            Value::PackedVarint(vec) => Some(vec.iter().map(|x| *x as u32).collect()),
            Value::Bytes(b) => parse_packed_varints(b).ok().map(|vec| vec.iter().map(|x| *x as u32).collect()),
            Value::BytesRef(off, len) => {
                 if let Some(buf) = &self.buffer {
                    let start = *off as usize;
                    let end = start + *len as usize;
                    if end <= buf.len() {
                        parse_packed_varints(&buf[start..end]).ok().map(|vec| vec.iter().map(|x| *x as u32).collect())
                    } else { None }
                 } else { None }
            },
            _ => None,
        })
    }

    #[napi]
    pub fn get_double_array(&self, id: u32) -> Option<Vec<f64>> {
        self.fields.get(&id).and_then(|v| match v {
            Value::Repeated(vec) => Some(vec.iter().filter_map(|x| if let Value::Bit64(i) = x { Some(f64::from_bits(*i)) } else { None }).collect()),
            _ => None,
        })
    }

    #[napi]
    pub fn get_float_array(&self, id: u32) -> Option<Vec<f64>> {
        self.fields.get(&id).and_then(|v| match v {
            Value::Repeated(vec) => Some(vec.iter().filter_map(|x| if let Value::Bit32(i) = x { Some(f32::from_bits(*i) as f64) } else { None }).collect()),
            _ => None,
        })
    }

    #[napi]
    pub fn get_bool_array(&self, id: u32) -> Option<Vec<bool>> {
        self.fields.get(&id).and_then(|v| match v {
            Value::Repeated(vec) => Some(vec.iter().filter_map(|x| if let Value::Varint(i) = x { Some(*i != 0) } else { None }).collect()),
            _ => None,
        })
    }

    #[napi]
    pub fn get_string_array(&self, id: u32) -> Option<Vec<String>> {
        self.fields.get(&id).and_then(|v| match v {
            Value::Repeated(vec) => Some(vec.iter().filter_map(|x| match x {
                Value::String(s) => Some(s.clone()),
                Value::Bytes(b) => String::from_utf8(b.clone()).ok(),
                Value::StringRef(off, len) | Value::BytesRef(off, len) => {
                    if let Some(buf) = &self.buffer {
                        let start = *off as usize;
                        let end = start + *len as usize;
                        if end <= buf.len() {
                            std::str::from_utf8(&buf[start..end]).ok().map(|s| s.to_string())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                },
                _ => None,
            }).collect()),
            _ => None,
        })
    }

    #[napi]
    pub fn get_bytes_array(&self, env: Env, id: u32) -> Option<Vec<JsBuffer>> {
        self.fields.get(&id).and_then(|v| match v {
            Value::Repeated(vec) => {
                let mut res = Vec::new();
                for item in vec {
                    match item {
                        Value::Bytes(b) => {
                            if let Ok(buf) = env.create_buffer_with_data(b.clone()) {
                                res.push(buf.into_raw());
                            }
                        },
                        Value::BytesRef(off, len) | Value::StringRef(off, len) => {
                            if let Some(buf) = &self.buffer {
                                let start = *off as usize;
                                let end = start + *len as usize;
                                if end <= buf.len() {
                                    if let Ok(buf) = env.create_buffer_with_data(buf[start..end].to_vec()) {
                                        res.push(buf.into_raw());
                                    }
                                }
                            }
                        },
                        _ => {}
                    }
                }
                Some(res)
            },
            _ => None,
        })
    }

    #[napi]
    pub fn get_packed_int32(&self, id: u32) -> Option<Vec<i32>> {
        self.fields.get(&id).and_then(|v| match v {
            Value::PackedVarint(vec) => Some(vec.iter().map(|x| *x as i32).collect()),
            Value::RepeatedVarint(vec) => Some(vec.iter().map(|x| *x as i32).collect()),
            Value::Bytes(b) => parse_packed_varints(b).ok().map(|vec| vec.iter().map(|x| *x as i32).collect()),
            Value::BytesRef(off, len) => {
                 if let Some(buf) = &self.buffer {
                    let start = *off as usize;
                    let end = start + *len as usize;
                    if end <= buf.len() {
                        parse_packed_varints(&buf[start..end]).ok().map(|vec| vec.iter().map(|x| *x as i32).collect())
                    } else { None }
                 } else { None }
            },
            _ => None,
        })
    }

    #[napi]
    pub fn get_packed_uint32(&self, id: u32) -> Option<Vec<u32>> {
        self.fields.get(&id).and_then(|v| match v {
            Value::PackedVarint(vec) => Some(vec.iter().map(|x| *x as u32).collect()),
            Value::RepeatedVarint(vec) => Some(vec.iter().map(|x| *x as u32).collect()),
            Value::Bytes(b) => parse_packed_varints(b).ok().map(|vec| vec.iter().map(|x| *x as u32).collect()),
            Value::BytesRef(off, len) => {
                 if let Some(buf) = &self.buffer {
                    let start = *off as usize;
                    let end = start + *len as usize;
                    if end <= buf.len() {
                        parse_packed_varints(&buf[start..end]).ok().map(|vec| vec.iter().map(|x| *x as u32).collect())
                    } else { None }
                 } else { None }
            },
            _ => None,
        })
    }

    #[napi]
    pub fn get_packed_int64(&self, env: Env, id: u32) -> Option<Vec<JsBigInt>> {
        self.fields.get(&id).and_then(|v| match v {
            Value::PackedVarint(vec) => {
                let mut res = Vec::new();
                for item in vec {
                    if let Ok(big) = env.create_bigint_from_i64(*item as i64) {
                        res.push(big);
                    }
                }
                Some(res)
            },
            _ => None,
        })
    }

    #[napi]
    pub fn get_packed_uint64(&self, env: Env, id: u32) -> Option<Vec<JsBigInt>> {
        self.fields.get(&id).and_then(|v| match v {
            Value::PackedVarint(vec) => {
                let mut res = Vec::new();
                for item in vec {
                    if let Ok(big) = env.create_bigint_from_u64(*item) {
                        res.push(big);
                    }
                }
                Some(res)
            },
            _ => None,
        })
    }

    #[napi]
    pub fn get_packed_sint64(&self, env: Env, id: u32) -> Option<Vec<JsBigInt>> {
        self.fields.get(&id).and_then(|v| match v {
            Value::PackedVarint(vec) => {
                let mut res = Vec::new();
                for item in vec {
                    let n = *item;
                    let decoded = (n >> 1) as i64 ^ -((n & 1) as i64);
                    if let Ok(big) = env.create_bigint_from_i64(decoded) {
                        res.push(big);
                    }
                }
                Some(res)
            },
            _ => None,
        })
    }

    #[napi]
    pub fn get_packed_bool(&self, id: u32) -> Option<Vec<bool>> {
        self.fields.get(&id).and_then(|v| match v {
            Value::PackedVarint(vec) => Some(vec.iter().map(|x| *x != 0).collect()),
            _ => None,
        })
    }

    // --- Packed Setters ---
    #[napi]
    pub fn set_packed_int32(&mut self, id: u32, value: Vec<i32>) {
        let v: Vec<u64> = value.into_iter().map(|x| x as i64 as u64).collect();
        self.fields.insert(id, Value::PackedVarint(v));
        self.invalidate_cache();
    }

    #[napi]
    pub fn set_packed_uint32(&mut self, id: u32, value: Vec<u32>) {
        let v: Vec<u64> = value.into_iter().map(|x| x as u64).collect();
        self.fields.insert(id, Value::PackedVarint(v));
        self.invalidate_cache();
    }

    #[napi]
    pub fn set_packed_float(&mut self, id: u32, value: Vec<f64>) {
        let v: Vec<u32> = value.into_iter().map(|x| (x as f32).to_bits()).collect();
        self.fields.insert(id, Value::PackedBit32(v));
        self.invalidate_cache();
    }

    #[napi]
    pub fn set_packed_double(&mut self, id: u32, value: Vec<f64>) {
        let v: Vec<u64> = value.into_iter().map(|x| x.to_bits()).collect();
        self.fields.insert(id, Value::PackedBit64(v));
        self.invalidate_cache();
    }

    #[napi]
    pub fn set_packed_bool(&mut self, id: u32, value: Vec<bool>) {
        let v: Vec<u64> = value.into_iter().map(|x| if x { 1 } else { 0 }).collect();
        self.fields.insert(id, Value::PackedVarint(v));
        self.invalidate_cache();
    }

    #[napi]
    pub fn set_packed_int64(&mut self, id: u32, value: Vec<BigInt>) {
        let v: Vec<u64> = value.into_iter().map(|x| x.get_u64().1).collect();
        self.fields.insert(id, Value::PackedVarint(v));
        self.invalidate_cache();
    }

    #[napi]
    pub fn set_packed_uint64(&mut self, id: u32, value: Vec<BigInt>) {
        let v: Vec<u64> = value.into_iter().map(|x| x.get_u64().1).collect();
        self.fields.insert(id, Value::PackedVarint(v));
        self.invalidate_cache();
    }

    #[napi]
    pub fn set_packed_sint32(&mut self, id: u32, value: Vec<i32>) {
        let v: Vec<u64> = value.into_iter().map(|x| {
            ((x << 1) ^ (x >> 31)) as u32 as u64
        }).collect();
        self.fields.insert(id, Value::PackedVarint(v));
        self.invalidate_cache();
    }

    #[napi]
    pub fn set_packed_sint64(&mut self, id: u32, value: Vec<BigInt>) {
        let v: Vec<u64> = value.into_iter().map(|x| {
            let n = x.get_i64().0;
            ((n << 1) ^ (n >> 63)) as u64
        }).collect();
        self.fields.insert(id, Value::PackedVarint(v));
        self.invalidate_cache();
    }

    #[napi]
    pub fn add_map_entry(&mut self, env: Env, id: u32, key: String, value: JsUnknown, key_type: u32, value_type: u32) -> napi::Result<()> {
        let map_key = parse_map_key(&key, key_type)?;
        let map_val = parse_map_value(&env, value, value_type)?;
        
        let val = self.fields.entry(id).or_insert_with(|| Value::Map(Box::new(FxHashMap::default())));
        if let Value::Map(map) = val {
            map.insert(map_key, map_val);
        } else {
            return Err(napi::Error::from_reason("Field is not a map"));
        }
        self.invalidate_cache();
        Ok(())
    }

    #[napi]
    pub fn add_map_entry_message(&mut self, env: Env, id: u32, key: String, value: &ManagedMessage, key_type: u32) -> napi::Result<()> {
        let map_key = parse_map_key(&key, key_type)?;
        let child_msg = value;
        let map_val = Value::Nested(Box::new(child_msg.clone()));
        
        let val = self.fields.entry(id).or_insert_with(|| Value::Map(Box::new(FxHashMap::default())));
        if let Value::Map(map) = val {
            map.insert(map_key, map_val);
        } else {
            return Err(napi::Error::from_reason("Field is not a map"));
        }
        self.invalidate_cache();
        Ok(())
    }

    #[napi]
    pub fn get_nested_array(&self, id: u32) -> Option<Vec<ManagedMessage>> {
        self.fields.get(&id).and_then(|v| match v {
            Value::Repeated(vec) => {
                let mut res = Vec::new();
                for item in vec {
                    match item {
                        Value::Nested(msg) => res.push(*msg.clone()),
                        Value::Group(msg) => res.push(*msg.clone()),
                        Value::Bytes(b) => {
                            if let Ok(msg) = ManagedMessage::decode_from_bytes(b) {
                                res.push(msg);
                            }
                        },
                        Value::BytesRef(off, len) | Value::NestedRef(off, len) => {
                             if let Some(buf) = &self.buffer {
                                let start = *off as usize;
                                let end = start + *len as usize;
                                if end <= buf.len() {
                                    if let Ok(msg) = ManagedMessage::decode_from_bytes(&buf[start..end]) {
                                        res.push(msg);
                                    }
                                }
                             }
                        },
                        _ => {}
                    }
                }
                if res.is_empty() { None } else { Some(res) }
            },
            _ => None,
        })
    }
}

fn compute_map_key_size(id: u32, key: &MapKey) -> usize {
    match key {
        MapKey::Varint(v) => {
            let tag = (id << 3) | 0;
            varint_size(tag as u64) + varint_size(*v)
        },
        MapKey::Bool(_) => {
            let tag = (id << 3) | 0;
            varint_size(tag as u64) + 1
        },
        MapKey::Bit32(_) => {
            let tag = (id << 3) | 5;
            varint_size(tag as u64) + 4
        },
        MapKey::Bit64(_) => {
            let tag = (id << 3) | 1;
            varint_size(tag as u64) + 8
        },
        MapKey::String(s) => {
            let tag = (id << 3) | 2;
            let len = s.len();
            varint_size(tag as u64) + varint_size(len as u64) + len
        },
    }
}

fn encode_map_key(buf: &mut Vec<u8>, id: u32, key: &MapKey) {
    match key {
        MapKey::Varint(v) => {
            let tag = (id << 3) | 0;
            write_varint32_fast(buf, tag);
            write_varint64_fast(buf, *v);
        },
        MapKey::Bool(v) => {
            let tag = (id << 3) | 0;
            write_varint32_fast(buf, tag);
            write_varint64_fast(buf, if *v { 1 } else { 0 });
        },
        MapKey::Bit32(v) => {
            let tag = (id << 3) | 5;
            write_varint32_fast(buf, tag);
            buf.extend_from_slice(&v.to_le_bytes());
        },
        MapKey::Bit64(v) => {
            let tag = (id << 3) | 1;
            write_varint32_fast(buf, tag);
            buf.extend_from_slice(&v.to_le_bytes());
        },
        MapKey::String(s) => {
            let tag = (id << 3) | 2;
            write_varint32_fast(buf, tag);
            let bytes = s.as_bytes();
            write_varint32_fast(buf, bytes.len() as u32);
            buf.extend_from_slice(bytes);
        },
    }
}

fn parse_map_key(s: &str, type_id: u32) -> napi::Result<MapKey> {
    match type_id {
        9 => Ok(MapKey::String(s.to_string())),
        8 => Ok(MapKey::Bool(s == "true")),
        5 | 17 => { // Int32, SInt32
            let v: i32 = s.parse().map_err(|_| napi::Error::from_reason("Invalid int32 key"))?;
            if type_id == 17 {
                let encoded = ((v << 1) ^ (v >> 31)) as u32;
                Ok(MapKey::Varint(encoded as u64))
            } else {
                Ok(MapKey::Varint(v as i64 as u64))
            }
        },
        13 => { // UInt32
            let v: u32 = s.parse().map_err(|_| napi::Error::from_reason("Invalid uint32 key"))?;
            Ok(MapKey::Varint(v as u64))
        },
        3 | 18 => { // Int64, SInt64
            let v: i64 = s.parse().map_err(|_| napi::Error::from_reason("Invalid int64 key"))?;
            if type_id == 18 {
                let encoded = ((v << 1) ^ (v >> 63)) as u64;
                Ok(MapKey::Varint(encoded))
            } else {
                Ok(MapKey::Varint(v as u64))
            }
        },
        4 => { // UInt64
            let v: u64 = s.parse().map_err(|_| napi::Error::from_reason("Invalid uint64 key"))?;
            Ok(MapKey::Varint(v))
        },
        7 => { // Fixed32
             let v: u32 = s.parse().map_err(|_| napi::Error::from_reason("Invalid fixed32 key"))?;
             Ok(MapKey::Bit32(v))
        },
        15 => { // SFixed32
             let v: i32 = s.parse().map_err(|_| napi::Error::from_reason("Invalid sfixed32 key"))?;
             Ok(MapKey::Bit32(v as u32))
        },
        6 => { // Fixed64
             let v: u64 = s.parse().map_err(|_| napi::Error::from_reason("Invalid fixed64 key"))?;
             Ok(MapKey::Bit64(v))
        },
        16 => { // SFixed64
             let v: i64 = s.parse().map_err(|_| napi::Error::from_reason("Invalid sfixed64 key"))?;
             Ok(MapKey::Bit64(v as u64))
        },
        _ => Err(napi::Error::from_reason("Unsupported map key type")),
    }
}

fn parse_map_value(env: &Env, val: JsUnknown, type_id: u32) -> napi::Result<Value> {
    match type_id {
        1 => { // Double
            let v: f64 = val.coerce_to_number()?.get_double()?;
            Ok(Value::Bit64(u64::from_le_bytes(v.to_le_bytes())))
        },
        2 => { // Float
            let v: f64 = val.coerce_to_number()?.get_double()?;
            let f = v as f32;
            Ok(Value::Bit32(u32::from_le_bytes(f.to_le_bytes())))
        },
        3 | 18 => { // Int64, SInt64
            // Expect BigInt or Number
            let v: i64 = if val.get_type()? == ValueType::BigInt {
                let big: JsBigInt = unsafe { val.cast() };
                big.get_i64()?.0
            } else {
                val.coerce_to_number()?.get_double()? as i64
            };
            if type_id == 18 {
                let encoded = ((v << 1) ^ (v >> 63)) as u64;
                Ok(Value::Varint(encoded))
            } else {
                Ok(Value::Varint(v as u64))
            }
        },
        4 => { // UInt64
            let v: u64 = if val.get_type()? == ValueType::BigInt {
                let big: JsBigInt = unsafe { val.cast() };
                big.get_u64()?.0
            } else {
                val.coerce_to_number()?.get_double()? as u64
            };
            Ok(Value::Varint(v))
        },
        5 | 17 | 14 => { // Int32, SInt32, Enum
            let v: i32 = val.coerce_to_number()?.get_int32()?;
            if type_id == 17 {
                let encoded = ((v << 1) ^ (v >> 31)) as u32;
                Ok(Value::Varint(encoded as u64))
            } else {
                Ok(Value::Varint(v as i64 as u64))
            }
        },
        13 => { // UInt32
            let v: u32 = val.coerce_to_number()?.get_uint32()?;
            Ok(Value::Varint(v as u64))
        },
        8 => { // Bool
            let v: bool = val.coerce_to_bool()?.get_value()?;
            Ok(Value::Varint(if v { 1 } else { 0 }))
        },
        9 => { // String
            let s: JsString = val.coerce_to_string()?;
            Ok(Value::String(s.into_utf8()?.into_owned()?))
        },
        12 => { // Bytes
            if val.is_buffer()? {
                let buf: JsBuffer = unsafe { val.cast() };
                Ok(Value::Bytes(buf.into_value()?.to_vec()))
            } else {
                Err(napi::Error::from_reason("Expected Buffer for bytes"))
            }
        },
        6 | 16 => { // Fixed64, SFixed64
            let v: u64 = if val.get_type()? == ValueType::BigInt {
                let big: JsBigInt = unsafe { val.cast() };
                big.get_u64()?.0
            } else {
                val.coerce_to_number()?.get_double()? as u64
            };
            Ok(Value::Bit64(v))
        },
        7 | 15 => { // Fixed32, SFixed32
            let v: u32 = val.coerce_to_number()?.get_uint32()?;
            Ok(Value::Bit32(v))
        },
        11 => { // Message
            let obj = val.coerce_to_object()?;
            let msg = env.unwrap::<ManagedMessage>(&obj)?;
            Ok(Value::Nested(Box::new(msg.clone())))
        },
        _ => Err(napi::Error::from_reason("Unsupported map value type")),
    }
}

fn value_to_js(env: &Env, val: &Value) -> napi::Result<JsUnknown> {
    match val {
        Value::Varint(v) => env.create_bigint_from_i64(*v as i64).and_then(|x| x.into_unknown()),
        Value::Bit64(v) => env.create_bigint_from_u64(*v).and_then(|x| x.into_unknown()), 
        Value::Bit32(v) => env.create_uint32(*v).map(|x| x.into_unknown()),
        Value::String(s) => env.create_string(s).map(|x| x.into_unknown()),
        Value::Bytes(b) => env.create_buffer_with_data(b.clone()).map(|x| x.into_unknown()),
        Value::Nested(msg) => {
             let raw = unsafe { <ManagedMessage as ToNapiValue>::to_napi_value(env.raw(), msg.as_ref().clone()) }?;
             unsafe { Ok(JsUnknown::from_raw_unchecked(env.raw(), raw)) }
        },
        Value::StringRef(_, _) | Value::BytesRef(_, _) | Value::NestedRef(_, _) => {
            // Should be resolved before calling value_to_js or handle here if we pass ManagedMessage context
            // For now, return undefined or error as this function doesn't have access to the buffer
            // In a real implementation, we would need to pass the buffer or resolve it earlier.
            // However, get_map handles this by parsing bytes.
            env.get_undefined().map(|x| x.into_unknown())
        },
        _ => env.get_undefined().map(|x| x.into_unknown()),
    }
}

impl Clone for ManagedMessage {
    fn clone(&self) -> Self {
        Self {
            fields: self.fields.clone(),
            cached_size: AtomicUsize::new(self.cached_size.load(Ordering::Relaxed)),
            buffer: self.buffer.clone(),
        }
    }
}

fn map_key_to_string(key: &MapKey) -> String {
    match key {
        MapKey::String(s) => s.clone(),
        MapKey::Varint(v) => v.to_string(),
        MapKey::Bit32(v) => v.to_string(),
        MapKey::Bit64(v) => v.to_string(),
        MapKey::Bool(b) => b.to_string(),
    }
}

fn default_map_key(key_type: u32) -> MapKey {
    match key_type {
        9 => MapKey::String("".to_string()),
        8 => MapKey::Bool(false),
        7 | 15 => MapKey::Bit32(0),
        6 | 16 => MapKey::Bit64(0),
        _ => MapKey::Varint(0),
    }
}

fn default_map_value(value_type: u32) -> Value {
    match value_type {
        9 => Value::String("".to_string()),
        12 => Value::Bytes(vec![]),
        11 => Value::Nested(Box::new(ManagedMessage::new())),
        1 | 6 | 16 => Value::Bit64(0),
        2 | 7 | 15 => Value::Bit32(0),
        _ => Value::Varint(0),
    }
}

fn read_map_key(decoder: &mut Decoder, _wire_type: u32, key_type: u32) -> napi::Result<MapKey> {
    match key_type {
        5 | 13 | 17 | 3 | 4 | 18 | 8 => { // Varint types
             let v = decoder.read_varint()?;
             match key_type {
                 8 => Ok(MapKey::Bool(v != 0)),
                 _ => Ok(MapKey::Varint(v)),
             }
        },
        7 | 15 => { // Fixed32
             let v = decoder.read_u32()?;
             Ok(MapKey::Bit32(v))
        },
        6 | 16 => { // Fixed64
             let v = decoder.read_u64()?;
             Ok(MapKey::Bit64(v))
        },
        9 => { // String
             let len = decoder.read_varint()? as usize;
             let bytes = decoder.read_bytes(len)?;
             let s = String::from_utf8_lossy(&bytes).to_string();
             Ok(MapKey::String(s))
        },
        _ => Err(napi::Error::from_reason("Invalid map key type")),
    }
}

fn read_map_value(decoder: &mut Decoder, _wire_type: u32, value_type: u32) -> napi::Result<Value> {
    match value_type {
        1 => { // Double
             let v = decoder.read_u64()?;
             Ok(Value::Bit64(v))
        },
        2 => { // Float
             let v = decoder.read_u32()?;
             Ok(Value::Bit32(v))
        },
        5 | 13 | 17 | 3 | 4 | 18 | 14 => { // Varint types
             let v = decoder.read_varint()?;
             Ok(Value::Varint(v))
        },
        7 | 15 => { // Fixed32
             let v = decoder.read_u32()?;
             Ok(Value::Bit32(v))
        },
        6 | 16 => { // Fixed64
             let v = decoder.read_u64()?;
             Ok(Value::Bit64(v))
        },
        8 => { // Bool
             let v = decoder.read_varint()?;
             Ok(Value::Varint(v))
        },
        9 => { // String
             let len = decoder.read_varint()? as usize;
             let bytes = decoder.read_bytes(len)?;
             let s = String::from_utf8_lossy(&bytes).to_string();
             Ok(Value::String(s))
        },
        12 => { // Bytes
             let len = decoder.read_varint()? as usize;
             let bytes = decoder.read_bytes(len)?;
             Ok(Value::Bytes(bytes.to_vec()))
        },
        11 => { // Message
             let len = decoder.read_varint()? as usize;
             let bytes = decoder.read_bytes(len)?;
             let msg = ManagedMessage::decode_from_bytes(&bytes)?;
             Ok(Value::Nested(Box::new(msg)))
        },
        _ => Err(napi::Error::from_reason("Invalid map value type")),
    }
}

fn parse_map_entry_from_bytes(bytes: &[u8], key_type: u32, value_type: u32) -> napi::Result<(MapKey, Value)> {
    let mut decoder = Decoder::new(bytes);
    let mut key = None;
    let mut value = None;
    
    while !decoder.eof() {
        let tag = decoder.read_varint()? as u32;
        if tag == 0 { break; }
        let field_id = tag >> 3;
        let wire_type = tag & 7;
        
        if field_id == 1 {
            // Key
            key = Some(read_map_key(&mut decoder, wire_type, key_type)?);
        } else if field_id == 2 {
            // Value
            value = Some(read_map_value(&mut decoder, wire_type, value_type)?);
        } else {
            decoder.skip_field(wire_type)?;
        }
    }
    
    let k = key.unwrap_or_else(|| default_map_key(key_type));
    let v = value.unwrap_or_else(|| default_map_value(value_type));
    
    Ok((k, v))
}



fn parse_packed_varints(bytes: &[u8]) -> napi::Result<Vec<u64>> {
    let mut decoder = Decoder::new(bytes);
    let mut res = Vec::new();
    while !decoder.eof() {
        res.push(decoder.read_varint()?);
    }
    Ok(res)
}

