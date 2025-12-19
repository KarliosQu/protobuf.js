
const protobuf = require('./src/index');
const rprotobuf = require('./rprotobuf');
const { ManagedMessage } = rprotobuf;

// Define a simple proto
const proto = `
syntax = "proto3";
message Test {
    string name = 1;
    int32 id = 2;
    float score = 3;
    repeated int32 values = 4;
    Nested nested = 5;
}
message Nested {
    string title = 1;
}
`;

const root = protobuf.parse(proto).root;
const Test = root.lookupType("Test");

const payload = {
    name: "Test Name",
    id: 12345,
    score: 123.45,
    values: [1, 2, 3, 4, 5, 100, 200, 300],
    nested: { title: "Nested Title" }
};

const ITERATIONS = 100000;

// Generate valid buffer using ManagedMessage
const m = new ManagedMessage();
m.setString(1, payload.name);
m.setInt32(2, payload.id);
m.setFloat(3, payload.score);
const buffer = m.encode();
console.log("Valid buffer length:", buffer.length);
console.log("Valid buffer hex:", buffer.toString('hex'));

// Warmup
for (let i = 0; i < 100; i++) {
    Test.decode(buffer);
}

// Baseline Decode
const startPbDecode = process.hrtime.bigint();
for (let i = 0; i < ITERATIONS; i++) {
    Test.decode(buffer);
}
const endPbDecode = process.hrtime.bigint();
const pbDecodeTime = Number(endPbDecode - startPbDecode) / 1e6;
console.log(`Baseline JS Decode: ${pbDecodeTime.toFixed(2)}ms, ${(ITERATIONS/pbDecodeTime*1000).toFixed(0)} ops/sec`);
