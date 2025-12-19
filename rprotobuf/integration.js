const protobuf = require("../src/index");
const adapter = require("./adapter");
const { ManagedMessage } = require("./index");

// Override Type.prototype.setup to avoid code generation (new Function)
// and to inject Rust-backed implementations.

const originalSetup = protobuf.Type.prototype.setup;

protobuf.Type.prototype.setup = function() {
    // 1. Ensure fields are resolved (copied logic from original setup)
    for (let i = 0; i < this.fieldsArray.length; ++i) {
        this._fieldsArray[i].resolve();
    }

    // 2. Assign our generic implementations
    
    // ENCODE
    this.encode = function(message, writer) {
        // Convert to ManagedMessage (Rust)
        const managedMsg = adapter.fromObject(this, message);
        
        // Encode to Buffer
        const buffer = managedMsg.encode();
        
        if (!writer) {
            const w = protobuf.Writer.create();
            // Hack: Override finish to return our buffer directly
            w.finish = function() { return buffer; };
            return w;
        }

        // Write to writer
        if (writer.raw) {
            writer.raw(buffer);
        } else {
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
        
        const subBuffer = Buffer.from(buffer.subarray(start, end));
        
        // Decode using Rust
        // Use static decode method instead of constructor for decoding from buffer
        const managedMsg = ManagedMessage.decode(subBuffer);
        // console.log("DEBUG: ManagedMessage:", managedMsg); // Might crash if no toString
        
        // Convert to JS Object (Lazy Proxy)
        const jsObj = adapter.toObject(this, managedMsg, { useConstructors: false });

        // Check required fields (Disabled)
        /*
        for (let i = 0; i < this.fieldsArray.length; ++i) {
            const field = this._fieldsArray[i];
            if (field.required && !Object.prototype.hasOwnProperty.call(jsObj, field.name)) {
                throw new protobuf.util.ProtocolError("missing required '" + field.name + "'", { instance: jsObj });
            }
        }
        */
        
        reader.pos = end;
        return jsObj;
    };
    
    // FROM OBJECT
    this.fromObject = function(object) {
        if (object && object.$type === this) {
            return object;
        }
        const managedMsg = adapter.fromObject(this, object);
        return adapter.toObject(this, managedMsg, { useConstructors: true });
    };
    
    // TO OBJECT
    this.toObject = function(message, options) {
        return adapter.toObject(this, adapter.fromObject(this, message), options);
    };
    
    // VERIFY
    this.verify = function(message) {
        try {
            adapter.fromObject(this, message);
            return null;
        } catch (e) {
            return e.message;
        }
    };

    return this;
};
module.exports = protobuf;
