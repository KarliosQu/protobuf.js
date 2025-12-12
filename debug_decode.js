var rprotobuf = require("./rprotobuf");
var { ManagedMessage } = rprotobuf;

function createMessage() {
    var msg = new ManagedMessage();
    msg.setString(1, "a".repeat(20000)); // 20KB string
    msg.setInt32(2, 123);
    
    var inner = new ManagedMessage();
    inner.setInt32(1, 456);
    msg.setNested(3, inner);
    
    var packed = [];
    for(var i=0; i<5000; i++) packed.push(i); // 5000 ints
    msg.setPackedInt32(4, packed);
    
    return msg;
}

try {
    console.log("Creating message...");
    var msg = createMessage();
    var buf = msg.encode();
    console.log("Encoded length:", buf.length);

    console.log("Decoding message...");
    var decoded = ManagedMessage.decode(buf);
    
    console.log("Verifying...");
    console.log("String:", decoded.getString(1));
    console.log("Int32:", decoded.getInt32(2));
    
    var inner = decoded.getNested(3);
    console.log("Nested Int32:", inner ? inner.getInt32(1) : "null");
    
    var packed = decoded.getPackedInt32(4);
    console.log("Packed length:", packed ? packed.length : "null");
    console.log("Packed[0]:", packed ? packed[0] : "null");
    console.log("Packed[99]:", packed ? packed[99] : "null");

    console.log("Benchmarking Decode...");
    var start = Date.now();
    var runs = 100000;
    for (var i = 0; i < runs; i++) {
        ManagedMessage.decode(buf);
    }
    var end = Date.now();
    var duration = end - start;
    var ops = runs / (duration / 1000);
    console.log(`Ops/sec: ${ops.toFixed(2)}`);

} catch (e) {
    console.error("Error:", e);
}
