const protobuf = require('./src/index');
const originalEncode = protobuf.Type.prototype.encode;

console.log("Original encode:", originalEncode.toString().substring(0, 50) + "...");

try {
    const rprotobuf = require('./rprotobuf/index');
    console.log("Loaded rprotobuf/index");
    
    if (protobuf.Type.prototype.encode === originalEncode) {
        console.log("protobuf.Type.prototype.encode is UNCHANGED");
    } else {
        console.log("protobuf.Type.prototype.encode was CHANGED");
    }

    if (rprotobuf.ManagedMessage) {
        console.log("ManagedMessage is available");
        const msg = new rprotobuf.ManagedMessage();
        console.log("ManagedMessage instantiated successfully");
    } else {
        console.log("ManagedMessage is NOT available");
    }

} catch (e) {
    console.error("Error:", e);
}
