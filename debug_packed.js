var rprotobuf = require("../rprotobuf");
var { ManagedMessage } = rprotobuf;

function createPackedMessage() {
    var msg = new ManagedMessage();
    
    // Create a large array of int32s
    var ints = [];
    for (var i = 0; i < 10000; i++) {
        ints.push(i);
    }
    msg.setPackedInt32(1, ints);

    // Create a large array of doubles
    var doubles = [];
    for (var i = 0; i < 10000; i++) {
        doubles.push(i * 1.1);
    }
    msg.setPackedDouble(2, doubles);

    return msg;
}

try {
    console.log("Creating packed message...");
    var msg = createPackedMessage();
    console.log("Encoding packed message...");
    var buf = msg.encode();
    console.log("Encoded length:", buf.length);
    console.log("Success!");

    console.log("Benchmarking Packed...");
    var start = Date.now();
    var runs = 10000;
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
