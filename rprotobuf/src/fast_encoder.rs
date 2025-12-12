use napi::bindgen_prelude::*;
use napi::{JsObject, Env, JsUnknown, NapiValue, NapiRaw, JsBoolean};
use napi_derive::napi;
use std::sync::Mutex;
use std::collections::HashMap;
use once_cell::sync::Lazy;

// 指令集：告诉 Rust 如何从扁平化数组中取值并编码
#[derive(Clone, Debug)]
pub enum Op {
    Double(u32),
    Float(u32),
    Int32(u32),
    UInt32(u32),
    SInt32(u32),
    Bool(u32),
    Int64(u32),
    UInt64(u32),
    SInt64(u32),
    String(u32),
    Bytes(u32),
    Nested(u32, u32),
    RepeatedDouble(u32),
    RepeatedInt32(u32),
    RepeatedString(u32),
    RepeatedBool(u32),
    RepeatedBoolPacked(u32),
    // ... 其他 repeated
}

pub struct Schema {
    ops: Vec<Op>,
}

// 全局 Schema 注册表
static SCHEMAS: Lazy<Mutex<HashMap<u32, Schema>>> = Lazy::new(|| Mutex::new(HashMap::new()));
static NEXT_ID: Lazy<Mutex<u32>> = Lazy::new(|| Mutex::new(1));

use std::cell::RefCell;

thread_local! {
    static BUFFER: RefCell<Vec<u8>> = RefCell::new(vec![0u8; 65536]);
}

#[napi]
pub mod fast_encoder {
    use super::*;

    #[napi]
    pub fn register_schema(_ops_js: Vec<JsObject>) -> u32 {
        // Placeholder
        0 
    }
    
    // 重新设计：使用 Int32Array 传递 Schema 定义，避免 JSObject 解析开销
    #[napi]
    pub fn register_schema_fast(instructions: Vec<i32>) -> u32 {
        let mut ops: Vec<Op> = Vec::new();
        let mut i = 0;
        while i < instructions.len() {
            let op_type = instructions[i];
            let tag = instructions[i+1] as u32;
            // let index = instructions[i+2] as u32; // Removed index
            
            let op = match op_type {
                1 => Op::Double(tag),
                2 => Op::Float(tag),
                3 => Op::Int32(tag),
                4 => Op::UInt32(tag),
                5 => Op::SInt32(tag),
                6 => Op::Bool(tag),
                7 => Op::Int64(tag),
                8 => Op::UInt64(tag),
                9 => Op::SInt64(tag),
                10 => Op::String(tag),
                11 => Op::Bytes(tag),
                12 => {
                    let sub_id = instructions[i+2] as u32;
                    i += 1; // Extra arg
                    Op::Nested(tag, sub_id)
                },
                26 => Op::RepeatedBool(tag),
                27 => Op::RepeatedBoolPacked(tag),
                // ... others
                _ => panic!("Unknown op type {}", op_type),
            };
            ops.push(op);
            i += 2; // op_type + tag
        }
        
        let mut id_lock = NEXT_ID.lock().unwrap();
        let id = *id_lock;
        *id_lock += 1;
        
        SCHEMAS.lock().unwrap().insert(id, Schema { ops });
        id
    }

    #[napi]
    pub fn encode_fast(schema_id: u32, numerics: Float64Array, n_len: u32, longs: BigInt64Array, l_len: u32, strings: Uint8Array, s_len: u32, mut output: Uint8Array) -> napi::Result<u32> {
        let schemas = SCHEMAS.lock().unwrap();
        let schema = schemas.get(&schema_id).ok_or_else(|| napi::Error::from_reason("Schema not found"))?;
        
        let numerics_slice = &numerics.as_ref()[0..n_len as usize];
        let longs_slice = &longs.as_ref()[0..l_len as usize];
        let strings_slice = &strings.as_ref()[0..s_len as usize];
        
        // Direct write to output buffer
        let output_slice = output.as_mut();
        let mut pos = output_slice.len();
        
        // Cursors
        let mut n_pos = n_len as usize;
        let mut l_pos = l_len as usize;
        let mut s_pos = s_len as usize;
        
        match encode_reverse(output_slice, &mut pos, schema, numerics_slice, &mut n_pos, longs_slice, &mut l_pos, strings_slice, &mut s_pos, &schemas) {
            Ok(_) => Ok(pos as u32),
            Err(e) => Err(e),
        }
    }
}

fn encode_reverse(buf: &mut [u8], pos: &mut usize, schema: &Schema, numerics: &[f64], n_pos: &mut usize, longs: &[i64], l_pos: &mut usize, strings: &[u8], s_pos: &mut usize, schemas: &HashMap<u32, Schema>) -> napi::Result<()> {
    for op in schema.ops.iter().rev() {
        // Check for buffer underflow risk (approximate check for safety)
        if *pos < 20 { 
             return Err(napi::Error::from_reason("Output buffer too small"));
        }

        match op {
            Op::Double(tag) => {
                *n_pos -= 1;
                let val = numerics[*n_pos];
                *pos -= 8;
                buf[*pos..*pos+8].copy_from_slice(&val.to_le_bytes());
                write_varint32_reverse(buf, pos, *tag);
            },
            Op::Float(tag) => {
                *n_pos -= 1;
                let val = numerics[*n_pos] as f32;
                *pos -= 4;
                buf[*pos..*pos+4].copy_from_slice(&val.to_le_bytes());
                write_varint32_reverse(buf, pos, *tag);
            },
            Op::Int32(tag) => {
                *n_pos -= 1;
                let val = numerics[*n_pos] as i32;
                if val < 0 {
                    write_varint64_reverse(buf, pos, val as i64 as u64);
                } else {
                    write_varint32_reverse(buf, pos, val as u32);
                }
                write_varint32_reverse(buf, pos, *tag);
            },
            Op::UInt32(tag) => {
                *n_pos -= 1;
                let val = numerics[*n_pos] as u32;
                write_varint32_reverse(buf, pos, val);
                write_varint32_reverse(buf, pos, *tag);
            },
            Op::SInt32(tag) => {
                *n_pos -= 1;
                let val = numerics[*n_pos] as i32;
                let encoded = ((val << 1) ^ (val >> 31)) as u32;
                write_varint32_reverse(buf, pos, encoded);
                write_varint32_reverse(buf, pos, *tag);
            },
            Op::Int64(tag) => {
                *l_pos -= 1;
                let val = longs[*l_pos];
                write_varint64_reverse(buf, pos, val as u64);
                write_varint32_reverse(buf, pos, *tag);
            },
            Op::SInt64(tag) => {
                *l_pos -= 1;
                let val = longs[*l_pos];
                let encoded = ((val << 1) ^ (val >> 63)) as u64;
                write_varint64_reverse(buf, pos, encoded);
                write_varint32_reverse(buf, pos, *tag);
            },
            Op::Bool(tag) => {
                *n_pos -= 1;
                let val = numerics[*n_pos];
                write_varint32_reverse(buf, pos, if val != 0.0 { 1 } else { 0 });
                write_varint32_reverse(buf, pos, *tag);
            },
            Op::String(tag) => {
                *n_pos -= 1;
                let len = numerics[*n_pos] as usize;
                *n_pos -= 1;
                let offset = numerics[*n_pos] as usize;
                
                if *pos < len { return Err(napi::Error::from_reason("Output buffer too small")); }
                *pos -= len;
                buf[*pos..*pos+len].copy_from_slice(&strings[offset..offset+len]);
                write_varint32_reverse(buf, pos, len as u32);
                write_varint32_reverse(buf, pos, *tag);
            },
            Op::Nested(tag, sub_id) => {
                *n_pos -= 1;
                let presence = numerics[*n_pos];
                if presence != 0.0 {
                    let end_pos = *pos;
                    let sub_schema = schemas.get(sub_id).unwrap();
                    encode_reverse(buf, pos, sub_schema, numerics, n_pos, longs, l_pos, strings, s_pos, schemas)?;
                    let sub_size = end_pos - *pos;
                    write_varint32_reverse(buf, pos, sub_size as u32);
                    write_varint32_reverse(buf, pos, *tag);
                }
            },
            Op::RepeatedBool(tag) => {
                 *n_pos -= 1;
                 let count = numerics[*n_pos] as usize;
                 for _ in 0..count {
                     *n_pos -= 1;
                     let val = numerics[*n_pos];
                     write_varint32_reverse(buf, pos, if val != 0.0 { 1 } else { 0 });
                     write_varint32_reverse(buf, pos, *tag);
                 }
            },
            Op::RepeatedBoolPacked(tag) => {
                 *n_pos -= 1;
                 let count = numerics[*n_pos] as usize;
                 if count > 0 {
                     let end_pos = *pos;
                     for _ in 0..count {
                         *n_pos -= 1;
                         let val = numerics[*n_pos];
                         write_varint32_reverse(buf, pos, if val != 0.0 { 1 } else { 0 });
                     }
                     let payload_size = end_pos - *pos;
                     write_varint32_reverse(buf, pos, payload_size as u32);
                     write_varint32_reverse(buf, pos, *tag);
                 }
            },
            _ => {}
        }
    }
    Ok(())
}

fn write_varint32_reverse(buf: &mut [u8], pos: &mut usize, mut v: u32) {
    let mut tmp = [0u8; 5];
    let mut len = 0;
    loop {
        if v >= 128 {
            tmp[len] = (v & 127) as u8 | 128;
            v >>= 7;
            len += 1;
        } else {
            tmp[len] = v as u8;
            len += 1;
            break;
        }
    }
    for i in (0..len).rev() {
        *pos -= 1;
        buf[*pos] = tmp[i];
    }
}

fn write_varint64_reverse(buf: &mut [u8], pos: &mut usize, mut v: u64) {
    let mut tmp = [0u8; 10];
    let mut len = 0;
    loop {
        if v >= 128 {
            tmp[len] = (v & 127) as u8 | 128;
            v >>= 7;
            len += 1;
        } else {
            tmp[len] = v as u8;
            len += 1;
            break;
        }
    }
    for i in (0..len).rev() {
        *pos -= 1;
        buf[*pos] = tmp[i];
    }
}