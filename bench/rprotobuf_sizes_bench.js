"use strict";

var newSuite = require("./suite");
var Long = require("long");
var rprotobuf;
try {
    rprotobuf = require("../../rprotobuf");
} catch (e) {
    console.error("Could not load rprotobuf:", e);
    process.exit(1);
}
var ManagedMessage = rprotobuf.ManagedMessage;

// Load protobuf.js static
var pbjsStaticCls = require("./data/static_pbjs.js").Test;

// Helper to populate ManagedMessage from payload
function createRMsg(payload) {
    var rMsg = new ManagedMessage();
    if (payload.string) rMsg.setString(1, payload.string);
    if (payload.uint32) rMsg.setUint32(2, payload.uint32);
    if (payload.float) rMsg.setFloat(4, payload.float);

    if (payload.inner) {
        var inner = new ManagedMessage();
        if (payload.inner.int32) inner.setInt32(1, payload.inner.int32);
        
        if (payload.inner.innerInner) {
            var innerInner = new ManagedMessage();
            if (payload.inner.innerInner.long) {
                var l = Long.fromValue(payload.inner.innerInner.long);
                innerInner.setInt64(1, BigInt(l.toString()));
            }
            if (payload.inner.innerInner.enum) innerInner.setInt32(2, payload.inner.innerInner.enum);
            if (payload.inner.innerInner.sint32) innerInner.setSint32(3, payload.inner.innerInner.sint32);
            inner.setNested(2, innerInner);
        }

        if (payload.inner.outer) {
            var outer = new ManagedMessage();
            if (payload.inner.outer.bool && payload.inner.outer.bool.length > 0) {
                // Optimize: use setPackedBool if available or loop
                // The proto definition says 'repeated bool bool = 1;' which is packed by default in proto3?
                // bench.proto is proto3.
                // Let's check if ManagedMessage has setPackedBool.
                // Yes, it has setPackedBool.
                outer.setPackedBool(1, payload.inner.outer.bool);
            }
            if (payload.inner.outer.double) outer.setDouble(2, payload.inner.outer.double);
            inner.setNested(3, outer);
        }
        rMsg.setNested(3, inner);
    }
    return rMsg;
}

function generatePayload(size) {
    var strLen = 10;
    var arrLen = 0;
    if (size === 'medium') {
        strLen = 2048; // 2KB
        arrLen = 1000;
    } else if (size === 'large') {
        strLen = 1024 * 100; // 100KB
        arrLen = 50000;
    }

    var payload = {
        string: "a".repeat(strLen),
        uint32: 100,
        float: 123.456,
        inner: {
            int32: 200,
            innerInner: {
                long: 123456789,
                enum: 1,
                sint32: -42
            },
            outer: {
                bool: Array(arrLen).fill(true),
                double: 789.123
            }
        }
    };
    return payload;
}

var sizes = ['small', 'medium', 'large'];

sizes.forEach(function(size) {
    console.log("\n--- Benchmarking Size: " + size + " ---");
    var payload = generatePayload(size);
    
    // Prepare objects
    var pbjsMsg = new pbjsStaticCls(payload); // Use constructor instead of fromObject
    var pbjsBuf = pbjsStaticCls.encode(pbjsMsg).finish();
    
    var rMsg = createRMsg(payload);
    var rBuf = rMsg.encode();

    var jsonStr = JSON.stringify(payload);
    var jsonBuf = Buffer.from(jsonStr);

    console.log("Payload size (JSON): " + jsonBuf.length + " bytes");
    console.log("Payload size (Proto): " + pbjsBuf.length + " bytes");

    newSuite("Encoding (" + size + ")")
    .add("protobuf.js (static)", function() {
        pbjsStaticCls.encode(pbjsMsg).finish();
    })
    .add("rprotobuf (NAPI)", function() {
        rMsg.encode();
    })
    .add("JSON.stringify", function() {
        JSON.stringify(payload);
    })
    .run();

    newSuite("Decoding (" + size + ")")
    .add("protobuf.js (static)", function() {
        pbjsStaticCls.decode(pbjsBuf);
    })
    .add("rprotobuf (NAPI)", function() {
        ManagedMessage.decode(pbjsBuf);
    })
    .add("JSON.parse", function() {
        JSON.parse(jsonStr);
    })
    .run();
});
