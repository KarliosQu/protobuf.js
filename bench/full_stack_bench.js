"use strict";

var newSuite  = require("./suite"),
    payload   = require("./data/bench.json");

var protobuf = require("..");
var rprotobuf = require("../../rprotobuf");

// Setup protobuf.js
var pbjsCls = protobuf.loadSync(require.resolve("./data/bench.proto")).resolveAll().lookup("Test");
var pbjsMsg = payload;
var pbjsBuf = pbjsCls.encode(pbjsMsg).finish();

console.log("Buffer size:", pbjsBuf.length);

// Setup NativeMessage
var { NativeMessage } = rprotobuf;

// Pre-warm / Verify
var msg = new NativeMessage();
msg.decode(pbjsBuf);
var s = msg.getString(1);
console.log("Native String (first 50 chars):", s ? s.substring(0, 50) : "null");

var b = msg.getBytes(1);
console.log("Native Bytes (first 10):", b ? Buffer.from(b).slice(0, 10) : "null");

var jsMsg = pbjsCls.decode(pbjsBuf);

newSuite("decoding")
.add("protobuf.js (Standard)", function() {
    pbjsCls.decode(pbjsBuf);
})
.add("Full Stack Rust (Zero Copy)", function() {
    var msg = new NativeMessage();
    msg.decode(pbjsBuf);
})
.run();

newSuite("access string (field 1)")
.add("JS Object Access", function() {
    var val = jsMsg.string;
})
.add("Rust Native Access (getString)", function() {
    var val = msg.getString(1);
})
.add("Rust Native Access (getBytes)", function() {
    var val = msg.getBytes(1);
})
.run();

newSuite("access nested message (Test.inner.int32)")
.add("JS Object Access", function() {
    var val = jsMsg.inner.int32;
})
.add("Rust Native Access", function() {
    var inner = msg.getMessage(3);
    if (inner) {
        var val = inner.getInt32(1);
    }
})
.run();

newSuite("access repeated field (Test.inner.outer.bool)")
.add("JS Object Access", function() {
    var len = jsMsg.inner.outer.bool.length;
    if (len > 0) {
        var val = jsMsg.inner.outer.bool[0];
    }
})
.add("Rust Native Access", function() {
    var inner = msg.getMessage(3);
    if (inner) {
        var outer = inner.getMessage(3);
        if (outer) {
            var len = outer.getCount(1);
            if (len > 0) {
                var val = outer.getBool(1, 0);
            }
        }
    }
})
.run();

newSuite("access uint32 (field 2)")
.add("JS Object Access", function() {
    var val = jsMsg.uint32;
})
.add("Rust Native Access", function() {
    var val = msg.getUint32(2);
})
.run();
