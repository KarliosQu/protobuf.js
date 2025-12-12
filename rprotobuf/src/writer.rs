use napi::bindgen_prelude::*;
use napi::{JsUnknown, JsObject, ValueType, JsFunction, Env, JsBuffer};
use napi_derive::napi;

#[napi]
pub struct Writer {
    stack: Vec<Vec<u8>>,
}

#[napi]
impl Writer {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            stack: vec![Vec::with_capacity(1024)],
        }
    }

    fn current(&mut self) -> &mut Vec<u8> {
        if self.stack.is_empty() {
            self.stack.push(Vec::new());
        }
        self.stack.last_mut().unwrap()
    }

    #[napi]
    pub fn uint32(&mut self, value: u32) -> &Self {
        let buf = self.current();
        write_varint32(buf, value);
        self
    }

    #[napi]
    pub fn int32(&mut self, value: i32) -> &Self {
        let buf = self.current();
        write_varint64(buf, value as i64 as u64);
        self
    }

    #[napi]
    pub fn sint32(&mut self, value: i32) -> &Self {
        self.uint32(((value << 1) ^ (value >> 31)) as u32)
    }

    #[napi]
    pub fn bool(&mut self, value: bool) -> &Self {
        let buf = self.current();
        buf.push(if value { 1 } else { 0 });
        self
    }

    #[napi]
    pub fn fixed32(&mut self, value: u32) -> &Self {
        let buf = self.current();
        buf.extend_from_slice(&value.to_le_bytes());
        self
    }

    #[napi]
    pub fn sfixed32(&mut self, value: i32) -> &Self {
        self.fixed32(value as u32)
    }

    #[napi]
    pub fn float(&mut self, value: f64) -> &Self {
        let val = value as f32;
        self.fixed32(val.to_bits())
    }

    #[napi]
    pub fn double(&mut self, value: f64) -> &Self {
        let buf = self.current();
        buf.extend_from_slice(&value.to_le_bytes());
        self
    }

    #[napi]
    pub fn string(&mut self, value: String) -> &Self {
        let buf = self.current();
        let bytes = value.as_bytes();
        write_varint32(buf, bytes.len() as u32);
        buf.extend_from_slice(bytes);
        self
    }

    #[napi]
    pub fn bytes(&mut self, env: Env, value: JsUnknown) -> napi::Result<&Self> {
        let buf = self.current();
        let global = env.get_global()?;
        let buffer_ctor: JsFunction = global.get_named_property("Buffer")?;
        
        let type_of = value.get_type()?;
        let buffer_res: JsObject = if type_of == ValueType::String {
             let args = vec![value, env.create_string("base64")?.into_unknown()];
             buffer_ctor.call(None, &args)?.try_into()?
        } else {
             buffer_ctor.call(None, &[value])?.try_into()?
        };
        
        let len: u32 = buffer_res.get_named_property("length")?;
        if len == 0 {
             write_varint32(buf, 0);
        } else {
             let js_buf: JsBuffer = buffer_res.into_unknown().try_into()?;
             let bytes = js_buf.into_value()?;
             write_varint32(buf, bytes.len() as u32);
             buf.extend_from_slice(&bytes);
        }
        Ok(self)
    }

    #[napi]
    pub fn raw(&mut self, value: Buffer) -> &Self {
        let buf = self.current();
        buf.extend_from_slice(value.as_ref());
        self
    }

    #[napi]
    pub fn fork(&mut self) -> &Self {
        self.stack.push(Vec::new());
        self
    }

    #[napi]
    pub fn ldelim(&mut self) -> &Self {
        let child = self.stack.pop().expect("No active fork");
        let parent = self.current();
        write_varint32(parent, child.len() as u32);
        parent.extend_from_slice(&child);
        self
    }

    #[napi]
    pub fn reset(&mut self) -> &Self {
        self.stack.clear();
        self.stack.push(Vec::new());
        self
    }

    #[napi]
    pub fn finish(&mut self) -> Buffer {
        let buf = self.stack.pop().unwrap_or_default();
        self.stack.push(Vec::new());
        buf.into()
    }

    #[napi(getter)]
    pub fn len(&mut self) -> u32 {
        self.current().len() as u32
    }
    
    #[napi]
    pub fn uint64(&mut self, value: JsUnknown) -> napi::Result<&Self> {
        let val = extract_u64(value)?;
        let buf = self.current();
        write_varint64(buf, val);
        Ok(self)
    }

    #[napi]
    pub fn int64(&mut self, value: JsUnknown) -> napi::Result<&Self> {
        let val = extract_i64(value)?;
        let buf = self.current();
        write_varint64(buf, val as u64);
        Ok(self)
    }

    #[napi]
    pub fn sint64(&mut self, value: JsUnknown) -> napi::Result<&Self> {
        let val = extract_i64(value)?;
        let encoded = (val << 1) ^ (val >> 63);
        let buf = self.current();
        write_varint64(buf, encoded as u64);
        Ok(self)
    }

    #[napi]
    pub fn fixed64(&mut self, value: JsUnknown) -> napi::Result<&Self> {
        let val = extract_u64(value)?;
        let buf = self.current();
        buf.extend_from_slice(&val.to_le_bytes());
        Ok(self)
    }

    #[napi]
    pub fn sfixed64(&mut self, value: JsUnknown) -> napi::Result<&Self> {
        let val = extract_i64(value)?;
        let buf = self.current();
        buf.extend_from_slice(&val.to_le_bytes());
        Ok(self)
    }

    #[napi]
    pub fn create() -> Self {
        Self::new()
    }
}

fn write_varint32(buf: &mut Vec<u8>, mut value: u32) {
    while value > 0x7F {
        buf.push((value as u8 & 0x7F) | 0x80);
        value >>= 7;
    }
    buf.push(value as u8);
}

fn write_varint64(buf: &mut Vec<u8>, mut value: u64) {
    while value > 0x7F {
        buf.push((value as u8 & 0x7F) | 0x80);
        value >>= 7;
    }
    buf.push(value as u8);
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
