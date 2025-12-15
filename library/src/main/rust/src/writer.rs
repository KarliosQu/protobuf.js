use napi::bindgen_prelude::*;
use napi::{JsUnknown, JsObject, ValueType, JsFunction, Env, JsBuffer};
use napi_derive::napi;
use crate::pool::{PooledBuffer, BUFFER_POOL};

#[napi]
pub struct Writer {
    stack: Vec<PooledBuffer>,
}

#[napi]
impl Writer {
    #[napi(constructor)]
    pub fn new() -> Self {
        Writer {
            stack: vec![BUFFER_POOL.acquire(1024)],
        }
    }

    fn current(&mut self) -> &mut Vec<u8> {
        self.stack.last_mut().unwrap().as_mut_vec()
    }

    #[napi]
    pub fn uint32(&mut self, value: u32) {
        let buf = self.current();
        write_varint32_fast(buf, value);
    }

    #[napi]
    pub fn int32(&mut self, value: i32) {
        let buf = self.current();
        write_varint64_fast(buf, value as i64 as u64);
    }

    #[napi]
    pub fn sint32(&mut self, value: i32) {
        self.uint32(((value << 1) ^ (value >> 31)) as u32);
    }

    #[napi]
    pub fn bool(&mut self, value: bool) {
        let buf = self.current();
        buf.push(if value { 1 } else { 0 });
    }

    #[napi]
    pub fn fixed32(&mut self, value: u32) {
        let buf = self.current();
        buf.extend_from_slice(&value.to_le_bytes());
    }

    #[napi]
    pub fn sfixed32(&mut self, value: i32) {
        self.fixed32(value as u32);
    }

    #[napi]
    pub fn float(&mut self, value: f64) {
        let val = value as f32;
        self.fixed32(val.to_bits());
    }

    #[napi]
    pub fn double(&mut self, value: f64) {
        let buf = self.current();
        buf.extend_from_slice(&value.to_le_bytes());
    }

    #[napi]
    pub fn string(&mut self, value: String) {
        let buf = self.current();
        let bytes = value.as_bytes();
        write_varint32_fast(buf, bytes.len() as u32);
        buf.extend_from_slice(bytes);
    }

    #[napi]
    pub fn bytes(&mut self, value: Buffer) {
        let buf = self.current();
        let bytes = value.as_ref();
        write_varint32_fast(buf, bytes.len() as u32);
        buf.extend_from_slice(bytes);
    }

    #[napi]
    pub fn raw(&mut self, value: Buffer) {
        let buf = self.current();
        buf.extend_from_slice(value.as_ref());
    }

    #[napi]
    pub fn fork(&mut self) {
        self.stack.push(BUFFER_POOL.acquire(1024));
    }

    #[napi]
    pub fn ldelim(&mut self) {
        if self.stack.len() < 2 {
            return;
        }
        let child = self.stack.pop().unwrap();
        let parent = self.current();
        write_varint32_fast(parent, child.len() as u32);
        parent.extend_from_slice(child.as_slice());
    }

    #[napi]
    pub fn reset(&mut self) {
        self.stack.truncate(1);
        if !self.stack.is_empty() {
            self.stack[0].as_mut_vec().clear();
        } else {
            self.stack.push(BUFFER_POOL.acquire(1024));
        }
    }

    #[napi]
    pub fn finish(&mut self, env: Env) -> Result<JsUnknown> {
        if self.stack.is_empty() {
             return Err(Error::from_reason("Stack empty"));
        }
        env.create_buffer_copy(self.stack[0].as_slice()).map(|b| b.into_unknown())
    }

    #[napi(getter)]
    pub fn len(&mut self) -> u32 {
        self.current().len() as u32
    }
    
    #[napi]
    pub fn uint64(&mut self, value: JsUnknown) -> napi::Result<()> {
        let val = extract_u64(value)?;
        let buf = self.current();
        write_varint64_fast(buf, val);
        Ok(())
    }

    #[napi]
    pub fn int64(&mut self, value: JsUnknown) -> napi::Result<()> {
        let val = extract_i64(value)?;
        let buf = self.current();
        write_varint64_fast(buf, val as u64);
        Ok(())
    }

    #[napi]
    pub fn sint64(&mut self, value: JsUnknown) -> napi::Result<()> {
        let val = extract_i64(value)?;
        let encoded = (val << 1) ^ (val >> 63);
        let buf = self.current();
        write_varint64_fast(buf, encoded as u64);
        Ok(())
    }

    #[napi]
    pub fn fixed64(&mut self, value: JsUnknown) -> napi::Result<()> {
        let val = extract_u64(value)?;
        let buf = self.current();
        buf.extend_from_slice(&val.to_le_bytes());
        Ok(())
    }

    #[napi]
    pub fn sfixed64(&mut self, value: JsUnknown) -> napi::Result<()> {
        let val = extract_i64(value)?;
        let buf = self.current();
        buf.extend_from_slice(&val.to_le_bytes());
        Ok(())
    }

    #[napi]
    pub fn create() -> Self {
        Self::new()
    }
}

#[napi]
pub fn encode_varint(value: u32) -> Buffer {
    let mut buf = Vec::with_capacity(5);
    write_varint32_fast(&mut buf, value);
    buf.into()
}

#[inline(always)]
pub fn write_varint32_fast(buf: &mut Vec<u8>, mut value: u32) {
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
pub fn write_varint64_fast(buf: &mut Vec<u8>, mut value: u64) {
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

fn extract_u64(value: JsUnknown) -> napi::Result<u64> {
    let type_of = value.get_type()?;
    match type_of {
        ValueType::Number => {
            let num: napi::JsNumber = value.try_into()?;
            let val: f64 = num.get_double()?;
            Ok(val as u64)
        },
        ValueType::Object => {
            let obj: JsObject = value.try_into()?;
            if let (Ok(low), Ok(high)) = (obj.get_named_property::<i32>("low"), obj.get_named_property::<i32>("high")) {
                 return Ok(((high as u64) << 32) | (low as u32 as u64));
            }
            Err(napi::Error::from_reason("Invalid object for uint64"))
        },
        ValueType::BigInt => {
            let bigint = unsafe { value.cast::<napi::JsBigInt>() };
            let val: u64 = bigint.try_into()?;
            Ok(val)
        },
        ValueType::String => {
            let js_str: napi::JsString = value.try_into()?;
            let s = js_str.into_utf8()?;
            let s_str = s.as_str()?;
            s_str.parse::<u64>().map_err(|_| napi::Error::from_reason("Failed to parse string as u64"))
        },
        _ => {
            Err(napi::Error::from_reason(format!("Unsupported type {:?} for uint64", type_of)))
        }
    }
}

fn extract_i64(value: JsUnknown) -> napi::Result<i64> {
    let type_of = value.get_type()?;
    match type_of {
        ValueType::Number => {
            let num: napi::JsNumber = value.try_into()?;
            let val: f64 = num.get_double()?;
            Ok(val as i64)
        },
        ValueType::Object => {
            let obj: JsObject = value.try_into()?;
            if let (Ok(low), Ok(high)) = (obj.get_named_property::<i32>("low"), obj.get_named_property::<i32>("high")) {
                 return Ok(((high as i64) << 32) | (low as u32 as i64));
            }
            Err(napi::Error::from_reason("Invalid object for int64"))
        },
        ValueType::BigInt => {
            let bigint = unsafe { value.cast::<napi::JsBigInt>() };
            let val: i64 = bigint.try_into()?;
            Ok(val)
        },
        ValueType::String => {
            let js_str: napi::JsString = value.try_into()?;
            let s = js_str.into_utf8()?;
            let s_str = s.as_str()?;
            s_str.parse::<i64>().map_err(|_| napi::Error::from_reason("Failed to parse string as i64"))
        },
        _ => {
            Err(napi::Error::from_reason(format!("Unsupported type {:?} for int64", type_of)))
        }
    }
}
