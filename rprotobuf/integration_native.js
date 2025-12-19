const protobuf = require("../src/index");
const { NativeType } = require("./index");

// Cache for Type -> NativeType mapping
const nativeTypeCache = new WeakMap();

function getOrBuildNativeType(type) {
    if (nativeTypeCache.has(type)) {
        return nativeTypeCache.get(type);
    }

    const native = new NativeType();
    console.log(`[NativeType] Building for ${type.name}, syntax=${type.syntax}`);
    nativeTypeCache.set(type, native); 

    // Ensure fields are resolved
    if (!type.resolved) {
        // type.fieldsArray getter triggers resolution
    }

    const fields = type.fieldsArray;
    for (let i = 0; i < fields.length; ++i) {
        const field = fields[i];
        let nestedNative = null;
        
        if (field.resolvedType) {
            // It's a message or enum
            if (field.resolvedType instanceof protobuf.Type) {
                nestedNative = getOrBuildNativeType(field.resolvedType);
            }
        }

        // Map type
        let fieldType = field.type;
        if (field.resolvedType instanceof protobuf.Enum) {
            fieldType = "uint32"; 
        } else if (field.resolvedType instanceof protobuf.Type) {
            if (field.resolvedType.group) {
                fieldType = "group";
            } else {
                fieldType = "message";
            }
        }
        
        try {
            if (field.map) {
                 console.log(`[NativeType] Adding map field ${field.name} to ${type.name}, is_map=${field.map}, type=${typeof field.map}`);
            }
            if (field.partOf) {
                 console.log(`[NativeType] Field ${field.name} is part of oneof ${field.partOf.name}`);
            }
            console.log(`[NativeType] Adding field ${field.name} to ${type.name}`);
            native.addField(
                field.name,
                field.id,
                fieldType,
                field.repeated,
                field.required,
                field.map || false,
                field.keyType || null,
                nestedNative,
                field.partOf ? field.partOf.name : null
            );
        } catch (e) {
            console.warn(`[NativeType] Failed to add field ${field.name} to ${type.name}: ${e.message}`);
        }
    }
    
    return native;
}

const originalSetup = protobuf.Type.prototype.setup;

protobuf.Type.prototype.setup = function() {
    // Call original setup to ensure everything is resolved and _ctor is created
    const result = originalSetup.call(this);
    
    // Override encode
    this.encode = function(message, writer) {
        if (!writer) writer = protobuf.Writer.create();
        
        try {
            const native = getOrBuildNativeType(this);
            
            if (process.env.HYBRID_ENCODE) {
                // Hybrid Encoding Logic
                let ops = new Int32Array(1024); // Pre-allocate, maybe resize if needed
                const refs = [];
                let opIdx = 0;
                
                const fields = this.fieldsArray;
                for (let i = 0; i < fields.length; ++i) {
                    const field = fields[i];
                    const val = message[field.name];
                    
                    if (val === undefined || val === null) continue;
                    if (field.repeated && val.length === 0) continue;
                    
                    // Check resize
                    if (opIdx + 10 > ops.length) {
                        const newOps = new Int32Array(ops.length * 2);
                        newOps.set(ops);
                        ops = newOps;
                    }

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

                    if (field.repeated) {
                        for (let j = 0; j < val.length; ++j) {
                            // Check resize
                            if (opIdx + 10 > ops.length) {
                                const newOps = new Int32Array(ops.length * 2);
                                newOps.set(ops);
                                ops = newOps;
                            }

                            const v = val[j];
                            ops[opIdx++] = i; // Field Index
                            
                            if (field.type === "string") {
                                ops[opIdx++] = 2;
                                ops[opIdx++] = refs.push(v) - 1;
                            } else if (field.type === "bytes") {
                                ops[opIdx++] = 3;
                                ops[opIdx++] = refs.push(v) - 1;
                            } else if (field.resolvedType instanceof protobuf.Type) {
                                if (field.resolvedType.group) {
                                    ops[opIdx++] = 10;
                                    ops[opIdx++] = refs.push(v) - 1;
                                } else {
                                    ops[opIdx++] = 4;
                                    // Recursive Hybrid Encoding for Repeated Fields
                                    try {
                                        const subBuffer = field.resolvedType.encode(v).finish();
                                        ops[opIdx++] = refs.push(subBuffer) - 1;
                                    } catch (e) {
                                        ops[opIdx++] = refs.push(v) - 1;
                                    }
                                }
                            } else if (field.type === "bool") {
                                ops[opIdx++] = 9;
                                ops[opIdx++] = v ? 1 : 0;
                            } else if (field.type === "int32" || field.type === "uint32" || field.type === "sint32") {
                                ops[opIdx++] = 0;
                                ops[opIdx++] = v;
                            } else if (field.type === "fixed32" || field.type === "sfixed32") {
                                ops[opIdx++] = 5;
                                ops[opIdx++] = v;
                            } else if (field.type === "fixed64" || field.type === "sfixed64") {
                                ops[opIdx++] = 6;
                                ops[opIdx++] = refs.push(v) - 1;
                            } else if (field.type === "float") {
                                ops[opIdx++] = 7;
                                ops[opIdx++] = refs.push(v) - 1;
                            } else if (field.type === "double") {
                                ops[opIdx++] = 8;
                                ops[opIdx++] = refs.push(v) - 1;
                            } else {
                                // Fallback to ref for others (int64, uint64, sint64)
                                ops[opIdx++] = 1; // Treat as generic varint64/number ref
                                ops[opIdx++] = refs.push(v) - 1;
                            }
                        }
                    } else {
                        ops[opIdx++] = i; // Field Index
                        
                        if (field.type === "string") {
                            ops[opIdx++] = 2;
                            ops[opIdx++] = refs.push(val) - 1;
                        } else if (field.type === "bytes") {
                            ops[opIdx++] = 3;
                            ops[opIdx++] = refs.push(val) - 1;
                        } else if (field.resolvedType instanceof protobuf.Type) {
                            if (field.resolvedType.group) {
                                ops[opIdx++] = 10;
                                ops[opIdx++] = refs.push(val) - 1;
                            } else {
                                ops[opIdx++] = 4;
                                // Recursive Hybrid Encoding!
                                // If we can encode the sub-message here, we pass a Buffer instead of the object.
                                // This avoids the slow Schema-Driven encoding in Rust.
                                try {
                                    // Note: We need to be careful about infinite recursion if types are recursive.
                                    // But protobuf.js encode handles this by stack depth usually? 
                                    // Here we just call encode.
                                    const subBuffer = field.resolvedType.encode(val).finish();
                                    ops[opIdx++] = refs.push(subBuffer) - 1;
                                } catch (e) {
                                    // Fallback to object if something fails (e.g. circular ref handling issues?)
                                    ops[opIdx++] = refs.push(val) - 1;
                                }
                            }
                        } else if (field.type === "bool") {
                            ops[opIdx++] = 9;
                            ops[opIdx++] = val ? 1 : 0;
                        } else if (field.type === "int32" || field.type === "uint32" || field.type === "sint32") {
                            ops[opIdx++] = 0;
                            ops[opIdx++] = val;
                        } else if (field.type === "fixed32" || field.type === "sfixed32") {
                            ops[opIdx++] = 5;
                            ops[opIdx++] = val;
                        } else if (field.type === "fixed64" || field.type === "sfixed64") {
                            ops[opIdx++] = 6;
                            ops[opIdx++] = refs.push(val) - 1;
                        } else if (field.type === "float") {
                            ops[opIdx++] = 7;
                            ops[opIdx++] = refs.push(val) - 1;
                        } else if (field.type === "double") {
                            ops[opIdx++] = 8;
                            ops[opIdx++] = refs.push(val) - 1;
                        } else {
                            // Fallback to ref for others (int64, uint64, sint64)
                            ops[opIdx++] = 1; // Treat as generic varint64/number ref
                            ops[opIdx++] = refs.push(val) - 1;
                        }
                    }
                }
                
                const buffer = native.encodeHybrid(ops.subarray(0, opIdx), refs);
                
                if (writer.raw) {
                    writer.raw(buffer);
                } else {
                    const writeRaw = (val, buf, pos) => {
                        for (let i = 0; i < val.length; ++i) buf[pos + i] = val[i];
                    };
                    writer._push(writeRaw, buffer.length, buffer);
                }

            } else {
                // Original Native Encoding
                // console.log(`[NativeType] Encoding ${this.name}, keys: ${Object.keys(message)}`);
                const buffer = native.encode(message);
                
                if (writer.raw) {
                    writer.raw(buffer);
                } else {
                    const writeRaw = (val, buf, pos) => {
                        for (let i = 0; i < val.length; ++i) buf[pos + i] = val[i];
                    };
                    writer._push(writeRaw, buffer.length, buffer);
                }
            }
        } catch (e) {
            console.error(`[NativeType] Encode failed for ${this.name}, falling back to JS:`, e);
            throw e;
        }
        
        return writer;
    };

    const originalDecode = this.decode;
    this.decode = function(reader, length) {
        if (reader instanceof Uint8Array) {
             try {
                 const native = getOrBuildNativeType(this);
                 return native.decode(reader);
             } catch (e) {
                 // console.warn(`[NativeType] Decode failed for ${this.name}, falling back to JS: ${e.message}`);
                 return originalDecode.call(this, reader, length);
             }
        }
        return originalDecode.call(this, reader, length);
    };
    
    return result;
};

module.exports = protobuf;
