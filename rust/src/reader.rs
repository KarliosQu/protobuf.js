use napi::bindgen_prelude::*;
use napi::{JsUnknown, JsObject, JsNumber, JsBuffer};
use napi_derive::napi;
use std::convert::TryInto;
use std::ffi::c_char;

#[napi]
pub struct Reader {
    pub(crate) buf: Vec<u8>,
    pub pos: u32,
    pub len: u32,
}

#[napi]
impl Reader {
    #[napi(constructor)]
    pub fn new(env: Env, buffer: JsUnknown) -> napi::Result<Self> {
        let buf: Vec<u8> = if buffer.is_buffer()? {
            let obj = unsafe { JsObject::from_raw_unchecked(env.raw(), buffer.raw()) };
            let len: u32 = obj.get_named_property("length")?;
            if len == 0 {
                Vec::new()
            } else {
                let js_buf: JsBuffer = buffer.try_into()?;
                js_buf.into_value()?.to_vec()
            }
        } else if buffer.is_array()? {
            // Handle array of numbers
            let obj: JsObject = buffer.try_into()?;
            let len = obj.get_array_length()?;
            let mut vec = Vec::with_capacity(len as usize);
            for i in 0..len {
                let val: JsNumber = obj.get_element(i)?;
                vec.push(val.get_uint32()? as u8);
            }
            vec
        } else {
            // Try to treat as Uint8Array or similar if possible, but Buffer covers most
            // If it's a plain array, we handled it.
            return Err(napi::Error::from_reason("Illegal buffer"));
        };

        let len = buf.len() as u32;
        Ok(Self {
            buf,
            pos: 0,
            len,
        })
    }

    #[napi(factory)]
    pub fn create(env: Env, buffer: JsUnknown) -> napi::Result<Self> {
        Self::new(env, buffer)
    }

    #[napi(getter)]
    pub fn get_buf(&self) -> Buffer {
        self.buf.clone().into()
    }

    #[napi]
    pub fn uint32(&mut self) -> napi::Result<u32> {
        let val = self.read_varint32()?;
        Ok(val)
    }

    #[napi]
    pub fn int32(&mut self) -> napi::Result<i32> {
        let val = self.read_varint32()?;
        Ok(val as i32)
    }

    #[napi]
    pub fn sint32(&mut self) -> napi::Result<i32> {
        let val = self.read_varint32()?;
        Ok(((val >> 1) as i32) ^ (-((val & 1) as i32)))
    }

    #[napi]
    pub fn bool(&mut self) -> napi::Result<bool> {
        let val = self.read_varint32()?;
        Ok(val != 0)
    }

    #[napi]
    pub fn fixed32(&mut self) -> napi::Result<u32> {
        if self.pos + 4 > self.len {
            return Err(napi::Error::from_reason("index out of range"));
        }
        let p = self.pos as usize;
        let bytes = &self.buf[p..p+4];
        let val = u32::from_le_bytes(bytes.try_into().unwrap());
        self.pos += 4;
        Ok(val)
    }

    #[napi]
    pub fn sfixed32(&mut self) -> napi::Result<i32> {
        let val = self.fixed32()?;
        Ok(val as i32)
    }

    #[napi]
    pub fn float(&mut self) -> napi::Result<f64> {
        if self.pos + 4 > self.len {
            return Err(napi::Error::from_reason("index out of range"));
        }
        let p = self.pos as usize;
        let bytes = &self.buf[p..p+4];
        let val = f32::from_le_bytes(bytes.try_into().unwrap());
        self.pos += 4;
        Ok(val as f64)
    }

    #[napi]
    pub fn double(&mut self) -> napi::Result<f64> {
        if self.pos + 8 > self.len {
            return Err(napi::Error::from_reason("index out of range"));
        }
        let p = self.pos as usize;
        let bytes = &self.buf[p..p+8];
        let val = f64::from_le_bytes(bytes.try_into().unwrap());
        self.pos += 8;
        Ok(val)
    }

    #[napi]
    pub fn bytes(&mut self) -> napi::Result<Buffer> {
        let len = self.read_varint32()? as usize;
        if self.pos as usize + len > self.len as usize {
            return Err(napi::Error::from_reason("index out of range"));
        }
        let p = self.pos as usize;
        let sub = self.buf[p..p+len].to_vec();
        self.pos += len as u32;
        Ok(sub.into())
    }

    #[napi]
    pub fn string(&mut self) -> napi::Result<String> {
        let len = self.read_varint32()? as usize;
        if self.pos as usize + len > self.len as usize {
            return Err(napi::Error::from_reason("index out of range"));
        }
        let p = self.pos as usize;
        let s = String::from_utf8_lossy(&self.buf[p..p+len]).to_string();
        self.pos += len as u32;
        Ok(s)
    }

    #[napi]
    pub fn skip(&mut self, length: Option<u32>) -> napi::Result<&Self> {
        let len = length.unwrap_or(0);
        if len == 0 {
            return Ok(self);
        }
        if self.pos + len > self.len {
            return Err(napi::Error::from_reason("index out of range"));
        }
        self.pos += len;
        Ok(self)
    }

    #[napi]
    pub fn skip_type(&mut self, wire_type: u32) -> napi::Result<&Self> {
        match wire_type {
            0 => { self.read_varint32()?; },
            1 => { self.skip(Some(8))?; },
            2 => {
                let len = self.read_varint32()?;
                self.skip(Some(len))?;
            },
            3 => {
                loop {
                    let tag = self.read_varint32()?;
                    let wire = tag & 7;
                    if wire == 4 { break; }
                    self.skip_type(wire)?;
                }
            },
            5 => { self.skip(Some(4))?; },
            _ => return Err(napi::Error::from_reason(format!("invalid wire type {} at offset {}", wire_type, self.pos))),
        }
        Ok(self)
    }

    // 64-bit methods - returning BigInt for now
    #[napi]
    pub fn uint64(&mut self) -> napi::Result<BigInt> {
        let val = self.read_varint64()?;
        Ok(BigInt::from(val))
    }

    #[napi]
    pub fn int64(&mut self) -> napi::Result<BigInt> {
        let val = self.read_varint64()?;
        Ok(BigInt::from(val as i64))
    }

    #[napi]
    pub fn sint64(&mut self) -> napi::Result<BigInt> {
        let val = self.read_varint64()?;
        let decoded = (val >> 1) ^ (-( (val & 1) as i64 ) as u64);
        Ok(BigInt::from(decoded as i64))
    }

    #[napi]
    pub fn fixed64(&mut self) -> napi::Result<BigInt> {
        if self.pos + 8 > self.len {
            return Err(napi::Error::from_reason("index out of range"));
        }
        let p = self.pos as usize;
        let bytes = &self.buf[p..p+8];
        let val = u64::from_le_bytes(bytes.try_into().unwrap());
        self.pos += 8;
        Ok(BigInt::from(val))
    }

    #[napi]
    pub fn sfixed64(&mut self) -> napi::Result<BigInt> {
        if self.pos + 8 > self.len {
            return Err(napi::Error::from_reason("index out of range"));
        }
        let p = self.pos as usize;
        let bytes = &self.buf[p..p+8];
        let val = i64::from_le_bytes(bytes.try_into().unwrap());
        self.pos += 8;
        Ok(BigInt::from(val))
    }
}

impl Reader {
    pub(crate) fn read_varint32(&mut self) -> napi::Result<u32> {
        Ok(self.read_varint64()? as u32)
    }

    pub(crate) fn read_varint64(&mut self) -> napi::Result<u64> {
        let mut value: u64 = 0;
        let mut shift: u32 = 0;
        loop {
            if self.pos >= self.len {
                return Err(napi::Error::from_reason("index out of range"));
            }
            let b = self.buf[self.pos as usize];
            self.pos += 1;
            value |= ((b & 127) as u64) << shift;
            if b < 128 {
                break;
            }
            shift += 7;
            if shift >= 70 {
                 return Err(napi::Error::from_reason("invalid varint encoding"));
            }
        }
        Ok(value)
    }
}
