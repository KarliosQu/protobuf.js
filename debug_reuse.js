var rprotobuf = require("./rprotobuf");
var { ManagedMessage } = rprotobuf;

function createMessage() {
    var msg = new ManagedMessage();
    msg.setString(1, "a".repeat(100));
    msg.setInt32(2, 123);
    return msg;
}

try {
    console.log("Benchmarking Allocation vs Reuse...");
    
    // 1. Allocation
    var start = Date.now();
    var runs = 1000000;
    for (var i = 0; i < runs; i++) {
        var msg = new ManagedMessage();
        msg.setString(1, "test");
        msg.setInt32(2, 123);
    }
    var end = Date.now();
    var duration = end - start;
    var ops = runs / (duration / 1000);
    console.log(`Allocation Ops/sec: ${ops.toFixed(2)}`);

    // 2. Reuse
    var msg = new ManagedMessage();
    var start = Date.now();
    for (var i = 0; i < runs; i++) {
        msg.clear();
        msg.setString(1, "test");
        msg.setInt32(2, 123);
    }
    var end = Date.now();
    var duration = end - start;
    var ops = runs / (duration / 1000);
    console.log(`Reuse Ops/sec:      ${ops.toFixed(2)}`);

} catch (e) {
    console.error("Error:", e);
}
