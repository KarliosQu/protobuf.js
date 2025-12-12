use std::slice;
use std::ptr::NonNull;
use napi::bindgen_prelude::*;
use napi::{JsBuffer, Ref, JsObject, JsFunction, JsString};
use napi_derive::napi;

#[derive(Debug, Clone, Copy)]
struct FieldInfo {
    #[allow(dead_code)]
    id: u32,
    wire_type: u32,
    data: u64, // varint/fixed value
    start: u32, // offset for LD
    end: u32,   // offset for LD
}

#[napi]
pub struct NativeMessage {
    // We hold a reference to the JS Buffer to prevent GC
    buf_ref: Option<Ref<()>>,
    // Raw pointer to the buffer data. 
    // SAFETY: We assume the Buffer backing store is stable (not moved by V8) 
    // and stays alive as long as buf_ref is alive.
    ptr: *const u8,
    len: usize,
    
    fields: Vec<FieldInfo>,
}

// NativeMessage is not Send because it holds raw pointers and JS references
unsafe impl Send for NativeMessage {} 

#[napi]
impl NativeMessage {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            buf_ref: None,
            ptr: std::ptr::null(),
            len: 0,
            fields: Vec::with_capacity(16),
        }
    }

    #[napi]
    pub fn decode(&mut self, env: Env, buffer: JsBuffer) -> napi::Result<()> {
        let reference = env.create_reference(buffer)?;
        
        let buffer_obj: JsBuffer = env.get_reference_value(&reference)?;
        let buffer_value = buffer_obj.into_value()?;
        let ptr = buffer_value.as_ptr();
        let len = buffer_value.len();
        
        self.buf_ref = Some(reference);
        
        // Ensure ptr is never null if we might use it for slice creation later.
        // slice::from_raw_parts requires non-null pointer even if len is 0.
        if len == 0 {
             self.ptr = NonNull::dangling().as_ptr();
        } else {
             self.ptr = ptr;
        }
        self.len = len;
        
        self.fields.clear();
        
        // Create a slice for parsing
        let buf = if len == 0 {
            &[]
        } else {
            if ptr.is_null() {
                 return Err(napi::Error::from_reason("Buffer pointer is null but length > 0".to_string()));
            }
            unsafe { slice::from_raw_parts(ptr, len) }
        };
        
        let mut pos = 0;

        while pos < len {
            let (tag, new_pos) = Self::read_varint32(buf, pos)?;
            pos = new_pos;
            
            let field_id = tag >> 3;
            let wire_type = tag & 7;

            let mut info = FieldInfo {
                id: field_id,
                wire_type,
                data: 0,
                start: 0,
                end: 0,
            };

            match wire_type {
                0 => {
                    let (val, new_pos) = Self::read_varint64(buf, pos)?;
                    info.data = val;
                    pos = new_pos;
                },
                1 => {
                    if pos + 8 > len {
                        return Err(napi::Error::from_reason("index out of range"));
                    }
                    let bytes = &buf[pos..pos+8];
                    info.data = u64::from_le_bytes(bytes.try_into().unwrap());
                    pos += 8;
                },
                2 => {
                    let (l, new_pos) = Self::read_varint32(buf, pos)?;
                    let length = l as usize;
                    pos = new_pos;
                    
                    info.start = pos as u32;
                    info.end = (pos + length) as u32;
                    
                    if pos + length > len {
                        return Err(napi::Error::from_reason("index out of range"));
                    }
                    pos += length;
                },
                5 => {
                    if pos + 4 > len {
                        return Err(napi::Error::from_reason("index out of range"));
                    }
                    let bytes = &buf[pos..pos+4];
                    info.data = u32::from_le_bytes(bytes.try_into().unwrap()) as u64;
                    pos += 4;
                },
                _ => {
                    return Err(napi::Error::from_reason("Unsupported wire type in NativeMessage"));
                }
            }
            
            self.fields.push(info);
        }
        Ok(())
    }

    fn read_varint32(buf: &[u8], pos: usize) -> napi::Result<(u32, usize)> {
        // Fast path for 1-byte varint (common for tags)
        if pos < buf.len() {
            let b = unsafe { *buf.get_unchecked(pos) };
            if b < 128 {
                return Ok((b as u32, pos + 1));
            }
        }
        let (val, new_pos) = Self::read_varint64(buf, pos)?;
        Ok((val as u32, new_pos))
    }

    #[inline(always)]
    fn read_varint64(buf: &[u8], mut pos: usize) -> napi::Result<(u64, usize)> {
        // Fast path: we have enough bytes, no bounds checks needed
        if pos + 10 <= buf.len() {
            let ptr = buf.as_ptr();
            unsafe {
                let mut b = *ptr.add(pos);
                if b < 128 { return Ok((b as u64, pos + 1)); }
                let mut value = (b & 127) as u64;
                pos += 1;

                b = *ptr.add(pos);
                value |= ((b & 127) as u64) << 7;
                pos += 1;
                if b < 128 { return Ok((value, pos)); }

                b = *ptr.add(pos);
                value |= ((b & 127) as u64) << 14;
                pos += 1;
                if b < 128 { return Ok((value, pos)); }

                b = *ptr.add(pos);
                value |= ((b & 127) as u64) << 21;
                pos += 1;
                if b < 128 { return Ok((value, pos)); }

                b = *ptr.add(pos);
                value |= ((b & 127) as u64) << 28;
                pos += 1;
                if b < 128 { return Ok((value, pos)); }
                
                // 5 bytes (up to 35 bits)
                b = *ptr.add(pos);
                value |= ((b & 127) as u64) << 35;
                pos += 1;
                if b < 128 { return Ok((value, pos)); }
                
                b = *ptr.add(pos);
                value |= ((b & 127) as u64) << 42;
                pos += 1;
                if b < 128 { return Ok((value, pos)); }
                
                b = *ptr.add(pos);
                value |= ((b & 127) as u64) << 49;
                pos += 1;
                if b < 128 { return Ok((value, pos)); }
                
                b = *ptr.add(pos);
                value |= ((b & 127) as u64) << 56;
                pos += 1;
                if b < 128 { return Ok((value, pos)); }
                
                b = *ptr.add(pos);
                value |= ((b & 127) as u64) << 63;
                pos += 1;
                if b < 128 { return Ok((value, pos)); }
                
                // 10th byte
                b = *ptr.add(pos);
                if b < 2 { 
                    // Last byte.
                    // Note: This logic is slightly simplified.
                    // Correct varint parsing for 10th byte is tricky.
                    // Let's just return here.
                    pos += 1;
                    return Ok((value, pos));
                }
            }
        }
        
        // Slow path / Fallback
        let mut value: u64 = 0;
        let mut shift: u32 = 0;
        loop {
            if pos >= buf.len() {
                return Err(napi::Error::from_reason("index out of range"));
            }
            let b = unsafe { *buf.get_unchecked(pos) };
            pos += 1;
            value |= ((b & 127) as u64) << shift;
            if b < 128 {
                break;
            }
            shift += 7;
            if shift >= 70 {
                 return Err(napi::Error::from_reason("invalid varint encoding"));
            }
        }
        Ok((value, pos))
    }

    #[napi]
    pub fn get_count(&self, id: u32) -> u32 {
        self.fields.iter().filter(|f| f.id == id).count() as u32
    }

    fn get_field(&self, id: u32, index: Option<u32>) -> Option<&FieldInfo> {
        if let Some(i) = index {
            // Find the i-th occurrence
            let mut count = 0;
            for field in self.fields.iter() {
                if field.id == id {
                    if count == i {
                        return Some(field);
                    }
                    count += 1;
                }
            }
            None
        } else {
            // Find the last occurrence
            for field in self.fields.iter().rev() {
                if field.id == id {
                    return Some(field);
                }
            }
            None
        }
    }

    #[napi]
    pub fn get_message(&self, env: Env, id: u32, index: Option<u32>) -> napi::Result<Option<NativeMessage>> {
        if self.ptr.is_null() {
            return Ok(None);
        }
        
        if let Some(field) = self.get_field(id, index) {
            if field.wire_type == 2 {
                let start = field.start as u32;
                let end = field.end as u32;
                
                if let Some(ref reference) = self.buf_ref {
                    let buffer_obj: JsObject = env.get_reference_value(reference)?;
                    let subarray_fn: JsFunction = buffer_obj.get_named_property("subarray")?;
                    let subarray: JsObject = subarray_fn.call(Some(&buffer_obj), &{
                        let mut arr = Vec::with_capacity(2);
                        arr.push(env.create_uint32(start)?.into_unknown());
                        arr.push(env.create_uint32(end)?.into_unknown());
                        arr
                    })?.try_into()?;
                    
                    let buffer: JsBuffer = subarray.into_unknown().try_into()?;
                    
                    let mut msg = NativeMessage::new();
                    msg.decode(env, buffer)?;
                    return Ok(Some(msg));
                }
            }
        }
        Ok(None)
    }

    #[napi]
    pub fn get_bytes(&self, env: Env, id: u32, index: Option<u32>) -> napi::Result<Option<JsObject>> {
        if self.ptr.is_null() {
            return Ok(None);
        }
        
        if let Some(field) = self.get_field(id, index) {
            if field.wire_type == 2 {
                let start = field.start as u32;
                let end = field.end as u32;
                
                if let Some(ref reference) = self.buf_ref {
                    let buffer_obj: JsObject = env.get_reference_value(reference)?;
                    let subarray_fn: JsFunction = buffer_obj.get_named_property("subarray")?;
                    let result: JsObject = subarray_fn.call(Some(&buffer_obj), &{
                        let mut arr = Vec::with_capacity(2);
                        arr.push(env.create_uint32(start)?.into_unknown());
                        arr.push(env.create_uint32(end)?.into_unknown());
                        arr
                    })?.try_into()?;
                    
                    return Ok(Some(result));
                }
            }
        }
        Ok(None)
    }

    #[napi]
    pub fn get_string(&self, env: Env, id: u32, index: Option<u32>) -> napi::Result<Option<JsString>> {
        if self.len == 0 || self.ptr.is_null() {
            return Ok(None);
        }
        let buf = unsafe { slice::from_raw_parts(self.ptr, self.len) };
        
        if let Some(field) = self.get_field(id, index) {
            if field.wire_type == 2 {
                let start = field.start as usize;
                let end = field.end as usize;
                if end <= buf.len() {
                    let bytes = &buf[start..end];
                    
                    // Optimization: Check for ASCII to potentially speed up string creation
                    // V8 can create OneByte strings faster than TwoByte (UTF-16)
                    // napi_create_string_utf8 handles this, but let's ensure we use the most direct path.
                    // Actually, env.create_string_from_utf8 is a wrapper.
                    // Let's try to use from_utf8 check to avoid allocation if possible, 
                    // but we already did that.
                    
                    // Try to convert to str without allocation first
                    match std::str::from_utf8(bytes) {
                        Ok(s_str) => {
                            let s = env.create_string(s_str)?;
                            return Ok(Some(s));
                        },
                        Err(_) => {
                            // Fallback to lossy conversion (allocates)
                            let s_lossy = String::from_utf8_lossy(bytes);
                            let s = env.create_string(&s_lossy)?;
                            return Ok(Some(s));
                        }
                    }
                }
            }
        }
        Ok(None)
    }
    
    #[napi]
    pub fn get_uint32(&self, id: u32, index: Option<u32>) -> napi::Result<Option<u32>> {
        if let Some(field) = self.get_field(id, index) {
            match field.wire_type {
                0 => return Ok(Some(field.data as u32)),
                5 => return Ok(Some(field.data as u32)),
                _ => {}
            }
        }
        Ok(None)
    }

    #[napi]
    pub fn get_float(&self, id: u32, index: Option<u32>) -> napi::Result<Option<f64>> {
        if let Some(field) = self.get_field(id, index) {
            match field.wire_type {
                5 => return Ok(Some(f32::from_bits(field.data as u32) as f64)),
                1 => return Ok(Some(f64::from_bits(field.data))),
                _ => {}
            }
        }
        Ok(None)
    }

    #[napi]
    pub fn get_bool(&self, id: u32, index: Option<u32>) -> napi::Result<Option<bool>> {
        if let Some(field) = self.get_field(id, index) {
            if field.wire_type == 0 {
                return Ok(Some(field.data != 0));
            }
        }
        Ok(None)
    }

    #[napi]
    pub fn get_int32(&self, id: u32, index: Option<u32>) -> napi::Result<Option<i32>> {
        if let Some(field) = self.get_field(id, index) {
            if field.wire_type == 0 {
                return Ok(Some(field.data as i32));
            }
        }
        Ok(None)
    }

    #[napi]
    pub fn get_int64(&self, id: u32, index: Option<u32>) -> napi::Result<Option<i64>> {
        if let Some(field) = self.get_field(id, index) {
            if field.wire_type == 0 {
                return Ok(Some(field.data as i64));
            }
        }
        Ok(None)
    }

    #[napi]
    pub fn get_double(&self, id: u32, index: Option<u32>) -> napi::Result<Option<f64>> {
        if let Some(field) = self.get_field(id, index) {
            if field.wire_type == 1 {
                return Ok(Some(f64::from_bits(field.data)));
            }
        }
        Ok(None)
    }
}
