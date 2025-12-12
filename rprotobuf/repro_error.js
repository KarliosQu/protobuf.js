const path = require('path');
require('./scripts/test-patch'); // Apply patch
const protobuf = require(path.resolve(__dirname, '../protobuf.js-protobufjs-v7.2.4'));

var ProtocolError = protobuf.util.ProtocolError;

var root = new protobuf.Root().add(
    new protobuf.Type("Test").add(
        new protobuf.Field("foo", 1, "uint32", "optional")
    ).add(
        new protobuf.Field("bar", 2, "string", "required")
    )
);

var Test = root.lookup("Test");
var buf  = protobuf.util.newBuffer(2);
buf[0] = 1 << 3 | 0;
buf[1] = 0x02;

console.log("Buffer:", buf);

try {
    var msg = Test.decode(buf);
    console.log("Did not throw! Msg:", msg);
    console.log("Msg has bar?", Object.prototype.hasOwnProperty.call(msg, "bar"));
    console.log("Msg bar value:", msg.bar);
} catch (e) {
    console.log("Caught expected error:", e.message);
}
