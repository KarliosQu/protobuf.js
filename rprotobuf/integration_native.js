const protobuf = require("../src/index");
const { NativeType } = require("./index");

// Cache for Type -> NativeType mapping
const nativeTypeCache = new WeakMap();

function getOrBuildNativeType(type) {
    if (nativeTypeCache.has(type)) {
        return nativeTypeCache.get(type);
    }

    const native = new NativeType();
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
            native.addField(
                field.name,
                field.id,
                fieldType,
                field.repeated,
                field.required,
                field.map || false,
                field.keyType || null,
                nestedNative
            );
        } catch (e) {
            // console.warn(`[NativeType] Failed to add field ${field.name} to ${type.name}: ${e.message}`);
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
            const buffer = native.encode(message);
            
            if (writer.raw) {
                writer.raw(buffer);
            } else {
                const writeRaw = (val, buf, pos) => {
                    for (let i = 0; i < val.length; ++i) buf[pos + i] = val[i];
                };
                writer._push(writeRaw, buffer.length, buffer);
            }
        } catch (e) {
            // console.error(`[NativeType] Encode failed for ${this.name}, falling back to JS:`, e);
            throw e;
        }
        
        return writer;
    };
    
    return result;
};

module.exports = protobuf;
