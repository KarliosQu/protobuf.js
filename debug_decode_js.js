var protobuf = require("./index");

var root = protobuf.Root.fromJSON({
    nested: {
        Test: {
            fields: {
                s: { type: "string", id: 1 },
                i: { type: "int32", id: 2 },
                n: { type: "Nested", id: 3 },
                p: { type: "int32", id: 4, rule: "repeated", options: { packed: true } }
            }
        },
        Nested: {
            fields: {
                i: { type: "int32", id: 1 }
            }
        }
    }
});

var Test = root.lookupType("Test");

var payload = {
    s: "a".repeat(20000),
    i: 123,
    n: { i: 456 },
    p: []
};
for(var i=0; i<5000; i++) payload.p.push(i);

var msg = Test.create(payload);
var buf = Test.encode(msg).finish();

console.log("Encoded length:", buf.length);

console.log("Benchmarking JS Decode...");
var start = Date.now();
var runs = 100000;
for (var i = 0; i < runs; i++) {
    Test.decode(buf);
}
var end = Date.now();
var duration = end - start;
var ops = runs / (duration / 1000);
console.log(`Ops/sec: ${ops.toFixed(2)}`);
