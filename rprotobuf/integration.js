const protobuf = require("../protobuf.js-protobufjs-v7.2.4/src/index");
const adapter = require("./adapter");
const { ManagedMessage } = require("./index");

// Override Type.prototype.setup to avoid code generation (new Function)
// and to inject Rust-backed implementations.

const originalSetup = protobuf.Type.prototype.setup;

protobuf.Type.prototype.setup = function() {
    return originalSetup.call(this);
};
/*
    // 1. Ensure fields are resolved (copied logic from original setup)
    // We iterate fieldsArray to trigger the getter which initializes _fieldsArray
    // and then resolve each field.
    for (let i = 0; i < this.fieldsArray.length; ++i) {
        this._fieldsArray[i].resolve();
    }

    // 2. Assign our generic implementations
    
    // ENCODE
    this.encode = function(message, writer) {
        if (!writer) writer = protobuf.Writer.create();
        
        // Convert to ManagedMessage (Rust)
        // Note: adapter.fromObject returns a ManagedMessage
        const managedMsg = adapter.fromObject(this, message);
        
        // Encode to Buffer
        const buffer = managedMsg.encode();
        
        // Write to writer
        if (writer.raw) {
            // Rust Writer (rprotobuf)
            writer.raw(buffer);
        } else {
            // JS Writer (protobuf.js)
            // We use a custom operation to write raw bytes
            const writeRaw = (val, buf, pos) => {
                if (val.copy) val.copy(buf, pos);
                else for (let i = 0; i < val.length; ++i) buf[pos + i] = val[i];
            };
            writer._push(writeRaw, buffer.length, buffer);
        }
        
        return writer;
    };
    
    // DECODE
    this.decode = function(reader, length) {
        if (!(reader instanceof protobuf.Reader))
            reader = protobuf.Reader.create(reader);
            
        const buffer = reader.buf;
        const start = reader.pos;
        const end = length === undefined ? reader.len : start + length;
        
        // Slice buffer
        // Note: Buffer.subarray is fast (view)
        const subBuffer = buffer.subarray(start, end);
        
        // Decode using Rust
        const managedMsg = ManagedMessage.decode(subBuffer);
        
        // Convert to JS Object
        const jsObj = adapter.toObject(this, managedMsg, { useConstructors: true });

        // Check required fields
        for (let i = 0; i < this.fieldsArray.length; ++i) {
            const field = this._fieldsArray[i];
            if (field.required && !Object.prototype.hasOwnProperty.call(jsObj, field.name)) {
                throw new protobuf.util.ProtocolError("missing required '" + field.name + "'", { instance: jsObj });
            }
        }
        
        // Update reader position
        reader.pos = end;
        
        return jsObj;
    };
    
    // FROM OBJECT
    this.fromObject = function(object) {
        if (object && object.$type === this) {
            return object;
        }
        // We use the round-trip trick to validate and coerce
        // JS -> Rust -> JS
        const managedMsg = adapter.fromObject(this, object);
        return adapter.toObject(this, managedMsg, { useConstructors: true });
    };
    
    // TO OBJECT
    this.toObject = function(message, options) {
        // message is already a JS object (from decode or fromObject)
        // But toObject supports options like { enums: String, longs: String }
        return adapter.toObject(this, adapter.fromObject(this, message), options);
    };
    
    // VERIFY
    this.verify = function(message) {
        try {
            adapter.fromObject(this, message);
            return null; // Success
        } catch (e) {
            return e.message;
        }
    };

    return this;
};
*/
module.exports = protobuf;
