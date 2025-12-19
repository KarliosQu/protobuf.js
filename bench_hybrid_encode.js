
const rprotobuf = require('./rprotobuf');
// Load integration_native to patch protobuf.js
require('./rprotobuf/integration_native');

const protobuf = require('.');
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

// Warmup
for (let i = 0; i < 100; i++) {
    Test.encode(payload).finish();
}

// 1. Standard Native Encode (Baseline)
process.env.HYBRID_ENCODE = "";
const startStandard = process.hrtime.bigint();
for (let i = 0; i < ITERATIONS; i++) {
    Test.encode(payload).finish();
}
const endStandard = process.hrtime.bigint();
const standardTime = Number(endStandard - startStandard) / 1e6;
console.log(`Standard Native Encode: ${standardTime.toFixed(2)}ms, ${(ITERATIONS/standardTime*1000).toFixed(0)} ops/sec`);

// 2. Hybrid Encode
process.env.HYBRID_ENCODE = "1";
const startHybrid = process.hrtime.bigint();
for (let i = 0; i < ITERATIONS; i++) {
    Test.encode(payload).finish();
}
const endHybrid = process.hrtime.bigint();
const hybridTime = Number(endHybrid - startHybrid) / 1e6;
console.log(`Hybrid Encode: ${hybridTime.toFixed(2)}ms, ${(ITERATIONS/hybridTime*1000).toFixed(0)} ops/sec`);

// Verify correctness
const buffer = Test.encode(payload).finish();
const decoded = Test.decode(buffer);
console.log("Decoded name:", decoded.name);
