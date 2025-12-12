var ManagedMessage = require("./index").ManagedMessage;
const protobuf = require("../protobuf.js-protobufjs-v7.2.4/src/index");

const TYPE_ID_MAP = {
    "int32": 5,
    "uint32": 13,
    "sint32": 17,
    "int64": 3,
    "uint64": 4,
    "sint64": 18,
    "bool": 8,
    "enum": 14,
    "fixed64": 1,
    "sfixed64": 16,
    "double": 1,
    "string": 9,
    "bytes": 12,
    "message": 11,
    "fixed32": 7,
    "sfixed32": 15,
    "float": 2
};

function isAny(type) {
    return type.name === "Any" && type.parent && type.parent.name === "protobuf" && type.parent.parent && type.parent.parent.name === "google";
}

// DEBUG: Test native interaction
// try {
//     console.log("DEBUG: Testing addMapEntryMessage...");
//     console.log("ManagedMessage prototype keys:", Object.getOwnPropertyNames(ManagedMessage.prototype));
//     var m1 = new ManagedMessage();
//     var m2 = new ManagedMessage();
//     // 9 is string key type
//     m1.addMapEntryMessage(1, "test_key", m2, 9);
//     console.log("DEBUG: addMapEntryMessage success");
// } catch (e) {
//     console.error("DEBUG: addMapEntryMessage failed:", e);
// }

function fromObject(type, object) {
    if (isAny(type) && object["@type"]) {
        let typeUrl = object["@type"];
        const typeName = typeUrl.substring(typeUrl.lastIndexOf("/") + 1);
        const resolvedType = type.root.lookupTypeOrEnum(typeName);
        
        if (resolvedType) {
            // Normalize typeUrl to match protobuf.js behavior
            if (typeUrl.indexOf("/") < 0) {
                if (typeUrl.indexOf(".") >= 0) {
                    typeUrl = "/" + typeUrl.substring(typeUrl.lastIndexOf(".") + 1);
                } else {
                    typeUrl = "/" + typeUrl;
                }
            }

            const message = new ManagedMessage();
            message.setString(1, typeUrl); // type_url
            
            const nestedMsg = fromObject(resolvedType, object);
            const buffer = nestedMsg.encode();
            
            message.setBytes(2, buffer); // value
            return message;
        }
    }

    const message = new ManagedMessage();
    if (!object) return message;

    for (const field of type.fieldsArray) {
        // Only encode own properties (matches protobuf.js behavior)
        if (!Object.prototype.hasOwnProperty.call(object, field.name)) {
            continue;
        }

        const value = object[field.name];
        if (value === undefined || value === null) continue;

        if (field.repeated) {
            if (Array.isArray(value)) {
                // Only pack if type is packable
                const isPackable = field.packed && 
                                   field.type !== "string" && 
                                   field.type !== "bytes" && 
                                   field.type !== "message" && 
                                   field.type !== "group" &&
                                   (!field.resolvedType || field.resolvedType.constructor.name !== "Type");
                
                if (isPackable) {
                    setPackedField(message, field, value);
                } else {
                    for (const item of value) {
                        addRepeatedField(message, field, item);
                    }
                }
            }
        } else if (field.map) {
            const keyType = TYPE_ID_MAP[field.keyType] || 0;
            let valueType = TYPE_ID_MAP[field.type] || 0;
            if (valueType === 0 && field.resolvedType) {
                if (field.resolvedType.constructor.name === "Type") {
                    valueType = 11; // message
                } else if (field.resolvedType.constructor.name === "Enum") {
                    valueType = 14; // enum
                }
            }

            for (const key in value) {
                if (Object.prototype.hasOwnProperty.call(value, key)) {
                    let val = value[key];
                    if (field.type === "bytes") {
                        val = toBuffer(val);
                    } else if (field.resolvedType && field.resolvedType.constructor.name === "Enum") {
                        if (typeof val === 'string') {
                            val = field.resolvedType.values[val];
                        }
                    }
                    
                    if (valueType === 11) {
                        val = fromObject(field.resolvedType, val);
                        message.addMapEntryMessage(field.id, key, val, keyType);
                    } else {
                        message.addMapEntry(field.id, key, val, keyType, valueType);
                    }
                }
            }
        } else {
            setSingularField(message, field, value);
        }
    }
    return message;
}

/**
 * Converts a ManagedMessage instance to a plain JavaScript object.
 * @param {Type} type The reflected type
 * @param {ManagedMessage} message The ManagedMessage instance
 * @param {Object} [options] Conversion options
 * @returns {Object} The plain JavaScript object
 */
function toObject(type, message, options) {
    options = options || {};

    if (isAny(type) && options.json) {
        const typeUrl = message.getString(1);
        const value = message.getBytes(2);
        
        if (typeUrl && value) {
            const typeName = typeUrl.substring(typeUrl.lastIndexOf("/") + 1);
            const resolvedType = type.root.lookupTypeOrEnum(typeName);
            
            if (resolvedType) {
                const nestedMsg = ManagedMessage.decode(value);
                const nestedObj = toObject(resolvedType, nestedMsg, options);
                
                if (typeUrl.indexOf("/") < 0) {
                    nestedObj["@type"] = "type.googleapis.com/" + typeUrl;
                } else {
                    nestedObj["@type"] = typeUrl;
                }
                return nestedObj;
            }
        }
    }

    let object;
    if (options.useConstructors && type.ctor) {
        object = new type.ctor();
    } else {
        object = {};
    }

    for (const field of type.fieldsArray) {
        if (!field.resolvedType) {
            try { field.resolve(); } catch (e) {}
        }
        let value = getField(message, field, options);
        
        // Handle unset values
        if (value === null || value === undefined) {
            if (field.repeated && options.arrays) {
                object[field.name] = [];
            } else if (field.map && options.objects) {
                object[field.name] = {};
            } else if (options.defaults) {
                let defaultValue = field.defaultValue;
                if (field.type === "bytes" && !field.repeated && Array.isArray(defaultValue) && defaultValue.length === 0) {
                     defaultValue = Buffer.alloc(0);
                }
                // Force plain object for Long defaults to match test expectations
                if (defaultValue && typeof defaultValue.low === 'number' && typeof defaultValue.high === 'number') {
                     defaultValue = { low: defaultValue.low, high: defaultValue.high, unsigned: defaultValue.unsigned };
                }
                object[field.name] = defaultValue;
            }
            continue;
        }

        // Value is set (or present in ManagedMessage).
        if (!options.defaults) {
             // If field is optional (explicit presence), we should keep it even if it looks like a default
             if (field.optional && !field.repeated && !field.map) {
                 object[field.name] = processValue(value, field, options);
                 continue;
             }

             let isDefault = false;
             if (field.repeated) {
                 if (value.length === 0 && !options.arrays) isDefault = true;
             } else if (field.map) {
                 if (Object.keys(value).length === 0 && !options.objects) isDefault = true;
             } else {
                 // Scalar
                 if (field.type === 'string' && value === "") isDefault = true;
                 else if (field.type === 'bool' && value === false) isDefault = true;
                 else if (field.type === 'bytes' && value.length === 0) {
                     // Keep explicitly set empty bytes
                     isDefault = false;
                 }
                 else if ((field.type === 'int32' || field.type === 'uint32' || 
                      field.type === 'float' || field.type === 'double' || 
                      field.type === 'fixed32' || field.type === 'sfixed32' ||
                      field.type === 'sint32') && value === 0) isDefault = true;
                 else if (field.resolvedType && field.resolvedType.constructor.name === "Enum" && value === field.defaultValue) isDefault = true;
                 else if (typeof value === 'bigint' && value === 0n) isDefault = true;
             }
             

             if (isDefault) continue;
        }

        if (field.repeated) {
            if (Array.isArray(value)) {
                object[field.name] = value.map(item => processValue(item, field, options));
            }
        } else if (field.map) {
            const mapObj = {};
            const valueField = {
                type: field.type,
                resolvedType: field.resolvedType
            };
            for (const key in value) {
                mapObj[key] = processValue(value[key], valueField, options);
            }
            object[field.name] = mapObj;
        } else {
            object[field.name] = processValue(value, field, options);
        }
    }
    return object;
}

function toBigInt(val) {
    if (typeof val === 'bigint') return val;
    if (typeof val === 'number') return BigInt(Math.floor(val));
    if (typeof val === 'string') return BigInt(val);
    if (val && typeof val.low === 'number' && typeof val.high === 'number') {
        // Long.js or plain object
        if (typeof val.toString === 'function' && val.toString() !== '[object Object]') {
            return BigInt(val.toString());
        } else {
            const low = BigInt(val.low);
            const high = BigInt(val.high);
            return (high << 32n) | (low & 0xFFFFFFFFn);
        }
    }
    return BigInt(0);
}

function toBuffer(val) {
    if (val === null || val === undefined) return Buffer.alloc(0);
    if (Buffer.isBuffer(val)) return val;
    if (typeof val === 'string') return Buffer.from(val, 'base64');
    if (Array.isArray(val) || val instanceof Uint8Array) return Buffer.from(val);
    try {
        return Buffer.from(val);
    } catch (e) {
        return Buffer.alloc(0);
    }
}

// Helper functions

function setSingularField(message, field, value) {
    const id = field.id;
    if (!field.resolvedType) {
        try {
            field.resolve();
        } catch (e) {}
    }
    switch (field.type) {
        case "double": message.setDouble(id, value); break;
        case "float": message.setFloat(id, value); break;
        case "int32": message.setInt32(id, value); break;
        case "uint32": message.setUint32(id, value); break;
        case "sint32": message.setSint32(id, value); break;
        case "fixed32": message.setUint32(id, value); break;
        case "sfixed32": message.setInt32(id, value); break;
        case "int64": message.setInt64(id, toBigInt(value)); break;
        case "uint64": message.setUint64(id, toBigInt(value)); break;
        case "sint64": message.setSint64(id, toBigInt(value)); break;
        case "fixed64": message.setFixed64(id, toBigInt(value)); break;
        case "sfixed64": message.setSfixed64(id, toBigInt(value)); break;
        case "bool": message.setBool(id, value); break;
        case "string": message.setString(id, String(value)); break;
        case "bytes": message.setBytes(id, toBuffer(value)); break;
        default:
            if (field.resolvedType && field.resolvedType.constructor.name === "Type") {
                const nestedMessage = fromObject(field.resolvedType, value);
                
                // Check for group (field.group might be undefined, check resolvedType.group)
                const isGroup = field.group || (field.resolvedType && field.resolvedType.group);

                if (isGroup) {
                    message.setGroup(id, nestedMessage);
                } else {
                    message.setNested(id, nestedMessage);
                }
            } else if (field.resolvedType && field.resolvedType.constructor.name === "Enum") {
                let intVal = value;
                if (typeof value === 'string') {
                    intVal = field.resolvedType.values[value];
                }
                message.setInt32(id, intVal);
            }
            break;
    }
}

function addRepeatedField(message, field, value) {
    const id = field.id;
    switch (field.type) {
        case "double": message.addDouble(id, value); break;
        case "float": message.addFloat(id, value); break;
        case "int32": message.addInt32(id, value); break;
        case "uint32": message.addUint32(id, value); break;
        case "sint32": message.addSint32(id, value); break;
        case "fixed32": message.addUint32(id, value); break;
        case "sfixed32": message.addInt32(id, value); break;
        case "int64": message.addInt64(id, toBigInt(value)); break;
        case "uint64": message.addUint64(id, toBigInt(value)); break;
        case "sint64": message.addSint64(id, toBigInt(value)); break;
        case "fixed64": message.addFixed64(id, toBigInt(value)); break;
        case "sfixed64": message.addSfixed64(id, toBigInt(value)); break;
        case "bool": message.addBool(id, value); break;
        case "string": message.addString(id, String(value)); break;
        case "bytes": message.addBytes(id, toBuffer(value)); break;
        default:
            if (field.resolvedType && field.resolvedType.constructor.name === "Type") {
                const nestedMessage = fromObject(field.resolvedType, value);
                const isGroup = field.group || (field.resolvedType && field.resolvedType.group);
                // console.log(`DEBUG: addRepeatedField id=${id} isGroup=${isGroup} resolvedType.group=${field.resolvedType.group}`);
                if (isGroup) {
                    message.addGroup(id, nestedMessage);
                } else {
                    message.addNested(id, nestedMessage);
                }
            } else if (field.resolvedType && field.resolvedType.constructor.name === "Enum") {
                let intVal = value;
                if (typeof value === 'string') {
                    intVal = field.resolvedType.values[value];
                }
                message.addInt32(id, intVal);
            }
            break;
    }
}

function setPackedField(message, field, value) {
    const id = field.id;
    switch (field.type) {
        case "double": message.setPackedDouble(id, value); break;
        case "float": message.setPackedFloat(id, value); break;
        case "int32": message.setPackedInt32(id, value); break;
        case "uint32": message.setPackedUint32(id, value); break;
        case "sint32": message.setPackedSint32(id, value); break;
        case "fixed32": message.setPackedUint32(id, value); break;
        case "sfixed32": message.setPackedInt32(id, value); break;
        case "int64": message.setPackedInt64(id, value.map(toBigInt)); break;
        case "uint64": message.setPackedUint64(id, value.map(toBigInt)); break;
        case "sint64": message.setPackedSint64(id, value.map(toBigInt)); break;
        case "fixed64": message.setPackedUint64(id, value.map(toBigInt)); break;
        case "sfixed64": message.setPackedSint64(id, value.map(toBigInt)); break;
        case "bool": 
            message.setPackedBool(id, value);
            break;
        default:
            if (field.resolvedType && field.resolvedType.constructor.name === "Enum") {
                const intValues = value.map(v => {
                    if (typeof v === 'string') {
                        return field.resolvedType.values[v];
                    }
                    return v;
                });
                message.setPackedInt32(id, intValues);
            }
            break;
    }
}

function getField(message, field, options) {
    const id = field.id;
    if (field.repeated) {
        const isPackable = field.packed &&   
                           field.type !== "string" && 
                           field.type !== "bytes" && 
                           field.type !== "message" && 
                           field.type !== "group" &&
                           (!field.resolvedType || field.resolvedType.constructor.name !== "Type");

        if (isPackable) {
             switch (field.type) {
                case "double": return message.getPackedDouble(id);
                case "float": return message.getPackedFloat(id);
                case "int32": return message.getPackedInt32(id);
                case "uint32": return message.getPackedUint32(id);
                case "sint32": {
                    const raw = message.getPackedUint32(id);
                    if (!raw) return null;
                    return raw.map(n => (n >>> 1) ^ -(n & 1));
                }
                case "fixed32": return message.getPackedUint32(id);
                case "sfixed32": return message.getPackedInt32(id);
                case "int64": return message.getPackedInt64(id);
                case "uint64": return message.getPackedUint64(id);
                case "sint64": return message.getPackedSint64(id);
                case "fixed64": return message.getPackedUint64(id);
                case "sfixed64": return message.getPackedSint64(id);
                case "bool": {
                    let packed = message.getPackedBool(id);
                    if (!packed) {
                        const bytes = message.getBytes(id);
                        if (bytes && bytes.length > 0) {
                            packed = Array.from(bytes).map(b => !!b);
                        }
                    }
                    if (packed) return packed;
                    return message.getBoolArray(id);
                }
                default: 
                    if (field.resolvedType && field.resolvedType.constructor.name === "Enum") {
                        return message.getPackedInt32(id);
                    }
                    return null;
             }
        } else {
            switch (field.type) {
                case "double": return message.getDoubleArray(id);
                case "float": return message.getFloatArray(id);
                case "int32": return message.getInt32Array(id);
                case "uint32": return message.getUint32Array(id);
                case "bool": return message.getBoolArray(id);
                case "string": return message.getStringArray(id);
                case "bytes": return message.getBytesArray(id);
                default: 
                    if (field.resolvedType && field.resolvedType.constructor.name === "Type") {
                        const nestedArray = message.getNestedArray(id);
                        if (!nestedArray) return [];
                        return nestedArray;
                    }
                    return null; 
            }
        }
    } else if (field.map) {
        const keyType = TYPE_ID_MAP[field.keyType] || 0;
        let valueType = TYPE_ID_MAP[field.type] || 0;
        if (valueType === 0 && field.resolvedType) {
            if (field.resolvedType.constructor.name === "Type") {
                valueType = 11; // message
            } else if (field.resolvedType.constructor.name === "Enum") {
                valueType = 14; // enum
            }
        }
        return message.getMap(id, keyType, valueType);
    } else {
        switch (field.type) {
            case "double": return message.getDouble(id);
            case "float": return message.getFloat(id);
            case "int32": return message.getInt32(id);
            case "uint32": return message.getUint32(id);
            case "sint32": {
                const val = message.getUint32(id);
                if (val === null || val === undefined) return null;
                return (val >>> 1) ^ -(val & 1);
            }
            case "fixed32": return message.getUint32(id);
            case "sfixed32": return message.getInt32(id);
            case "int64": return message.getInt64(id);
            case "uint64": return message.getUint64(id);
            case "sint64": return message.getSint64(id);
            case "fixed64": return message.getUint64(id);
            case "sfixed64": return message.getSfixed64(id);
            case "bool": return message.getBool(id);
            case "string": return message.getString(id);
            case "bytes": return message.getBytes(id);
            default: 
                if (field.resolvedType && field.resolvedType.constructor.name === "Type") {
                    const nested = message.getNested(id);
                    return nested ? toObject(field.resolvedType, nested, options) : null;
                } else if (field.resolvedType && field.resolvedType.constructor.name === "Enum") {
                    return message.getInt32(id);
                }
                return null;
        }
    }
}

function fromBigInt(val, unsigned) {
    const low = Number(val & 0xFFFFFFFFn) | 0;
    const high = Number((val >> 32n) & 0xFFFFFFFFn) | 0;
    return { low: low, high: high, unsigned: unsigned };
}

function processValue(value, field, options) {
    if (value instanceof ManagedMessage) {
        if (field.resolvedType && field.resolvedType.constructor.name === "Type") {
            return toObject(field.resolvedType, value, options);
        }
    }

    if (typeof value === 'bigint') {
        if (options.longs === String) return value.toString();
        if (options.longs === Number) return Number(value);
        // Default: Long object (plain object representation)
        const unsigned = field.type ? field.type.indexOf('u') === 0 || field.type === 'fixed64' : false;
        
        if (protobuf.util.Long) {
            return protobuf.util.Long.fromValue(value.toString(), unsigned);
        }

        return fromBigInt(value, unsigned);
    }
    
    if (field.type === "bytes") {
        if (options.bytes === String) {
            if (Buffer.isBuffer(value)) return value.toString('base64');
            if (Array.isArray(value)) return Buffer.from(value).toString('base64');
            return String(value);
        }
        if (options.bytes === Array) {
            if (Buffer.isBuffer(value)) return Array.from(value);
            if (Array.isArray(value)) return value;
            return Array.from(Buffer.from(value));
        }
        // Default: Buffer
        if (Array.isArray(value)) value = Buffer.from(value);
        
        // Check for custom Buffer implementation
        if (protobuf.util.Buffer && protobuf.util.Buffer !== Buffer) {
            return protobuf.util.newBuffer(value);
        }
        
        return value;
    }

    if (field.resolvedType && field.resolvedType.constructor.name === "Enum") {
        if (options.enums === String) {
            return field.resolvedType.valuesById[value] || value;
        }
    }
    return value;
}

module.exports = {
    fromObject,
    toObject
};
