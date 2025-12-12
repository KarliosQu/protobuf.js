var rprotobuf = require("./rprotobuf");
var { ManagedMessage } = rprotobuf;

function createManagedMessage() {
    var msg = new ManagedMessage();
    msg.setString(1, "a".repeat(20000)); // Large string
    msg.setUint32(2, 123456);
    
    var inner = new ManagedMessage();
    inner.setInt32(1, -123);
    inner.setString(2, "nested string");
    
    msg.setNested(3, inner);
    msg.setNested(4, inner);
    msg.setNested(5, inner);
    return msg;
}

try {
    console.log("Creating message...");
    var msg = createManagedMessage();
    console.log("Encoding message...");
    var buf = msg.encode();
    console.log("Encoded length:", buf.length);
    console.log("Success!");

    console.log("Benchmarking...");
    var start = Date.now();
    var runs = 100000;
    for (var i = 0; i < runs; i++) {
        msg.encode();
    }
    var end = Date.now();
    var duration = end - start;
    var ops = runs / (duration / 1000);
    console.log(`Ops/sec: ${ops.toFixed(2)}`);

} catch (e) {
    console.error("Error:", e);
}
