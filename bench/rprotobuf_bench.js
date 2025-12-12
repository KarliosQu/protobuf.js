"use strict";

var newSuite  = require("./suite"),
    payload   = require("./data/bench.json");
var Long = require("long");

var Buffer_from = Buffer.from !== Uint8Array.from && Buffer.from || function(value, encoding) { return new Buffer(value, encoding); };

// protobuf.js dynamic
var pbjsCls = require("..").loadSync(require.resolve("./data/bench.proto")).resolveAll().lookup("Test");
var pbjsMsg = payload;
var pbjsBuf = pbjsCls.encode(pbjsMsg).finish();

// protobuf.js static
var pbjsStaticCls = require("./data/static_pbjs.js").Test;

// rprotobuf (NAPI)
var rprotobuf;
try {
    rprotobuf = require("../rprotobuf");
} catch (e) {
    console.error("Could not load rprotobuf:", e);
    process.exit(1);
}
var ManagedMessage = rprotobuf.ManagedMessage;

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
            if (payload.inner.outer.bool) {
                payload.inner.outer.bool.forEach(b => outer.addBool(1, b));
            }
            if (payload.inner.outer.double) outer.setDouble(2, payload.inner.outer.double);
            inner.setNested(3, outer);
        }
        rMsg.setNested(3, inner);
    }
    return rMsg;
}

var rMsg = createRMsg(payload);
// Warmup and verify
var rBuf;
try {
    rBuf = rMsg.encode();
} catch (e) {
    console.log("Encode failed:", e);
}

// JSON
var jsonMsg = payload;
var jsonStr = JSON.stringify(jsonMsg);
var jsonBuf = Buffer_from(jsonStr, "utf8");

newSuite("encoding")
.add("protobuf.js (reflect)", function() {
    pbjsCls.encode(pbjsMsg).finish();
})
.add("protobuf.js (static)", function() {
    pbjsStaticCls.encode(pbjsMsg).finish();
})
.add("rprotobuf (NAPI)", function() {
    rMsg.encode();
})
.add("JSON (string)", function() {
    JSON.stringify(jsonMsg);
})
.add("JSON (buffer)", function() {
    Buffer_from(JSON.stringify(jsonMsg), "utf8");
})
.run();

newSuite("decoding")
.add("protobuf.js (reflect)", function() {
    pbjsCls.decode(pbjsBuf);
})
.add("protobuf.js (static)", function() {
    pbjsStaticCls.decode(pbjsBuf);
})
.add("rprotobuf (NAPI)", function() {
    ManagedMessage.decode(pbjsBuf);
})
.add("JSON (string)", function() {
    JSON.parse(jsonStr);
})
.add("JSON (buffer)", function() {
    JSON.parse(jsonBuf.toString("utf8"));
})
.run();

newSuite("combined")
.add("protobuf.js (reflect)", function() {
    pbjsCls.decode(pbjsCls.encode(pbjsMsg).finish());
})
.add("protobuf.js (static)", function() {
    pbjsStaticCls.decode(pbjsStaticCls.encode(pbjsMsg).finish());
})
.add("rprotobuf (NAPI)", function() {
    ManagedMessage.decode(rMsg.encode());
})
.add("JSON (string)", function() {
    JSON.parse(JSON.stringify(jsonMsg));
})
.add("JSON (buffer)", function() {
    JSON.parse(Buffer_from(JSON.stringify(jsonMsg), "utf8").toString("utf8"));
})
.run();
