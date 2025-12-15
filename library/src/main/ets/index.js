import protobuf from "protobufjs";
import rprotobuf from 'librprotobuf.so';
import { fromObject, toObject } from "./adapter";

// Patch Type.encode
const originalEncode = protobuf.Type.prototype.encode;
protobuf.Type.prototype.encode = function(message, writer) {
    if (writer) {
        return originalEncode.call(this, message, writer);
    }
    try {
        const managed = fromObject(this, message);
        const buffer = managed.encode();
        // Return a mock Writer that returns this buffer on finish()
        return {
            finish: () => buffer,
            // Mock other methods to prevent crash if user chains simple calls, though unlikely for top-level encode
            ldelim: function() { return this; }, 
            reset: function() { return this; },
            fork: function() { return this; },
            // If user tries to write more data, it won't work as expected with this optimization
            // But standard usage Type.encode(msg).finish() is covered.
        };
    } catch (e) {
        // console.warn("rprotobuf encode failed, falling back to js:", e);
        return originalEncode.call(this, message, writer);
    }
};

// Patch Type.decode
const originalDecode = protobuf.Type.prototype.decode;
protobuf.Type.prototype.decode = function(buffer) {
    try {
        // Ensure buffer is Uint8Array
        if (buffer && !(buffer instanceof Uint8Array) && !Buffer.isBuffer(buffer)) {
             if (Array.isArray(buffer)) {
                buffer = new Uint8Array(buffer);
             }
        }
        
        // rprotobuf.ManagedMessage.decode expects a buffer
        const managed = rprotobuf.ManagedMessage.decode(buffer);
        
        // Convert back to JS object for compatibility
        // This ensures that the returned object behaves exactly like a standard protobuf.js Message
        return toObject(this, managed);
    } catch (e) {
        // console.warn("rprotobuf decode failed, falling back to js:", e);
        return originalDecode.call(this, buffer);
    }
};

// Export everything from protobuf to ensure drop-in compatibility
export default protobuf;

export const {
    Type, Field, Message, Root, Enum, Service, Method, Namespace,
    load, loadSync, encode, decode,
    Reader, Writer,
    util, reflection, build, configure,
    common, converter, decoder, encoder, verifier, wrappers,
    tokenize, parse,
    types,
    MapField, OneOf
} = protobuf;

// Export rprotobuf specific functionality for advanced usage
export const native = rprotobuf;
export { fromObject, toObject };
