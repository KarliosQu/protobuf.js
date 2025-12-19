const protobuf = require('.');
require('./rprotobuf/integration'); // Enable Drop-in mode

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
    name: "Test Message",
    id: 12345,
    score: 99.9,
    values: Array.from({length: 100}, (_, i) => i),
    nested: { title: "Nested Title" }
};

const buffer = Test.encode(payload).finish();
console.log("Buffer length:", buffer.length);

const ITERATIONS = 1;

console.log("Starting Lazy Proxy Benchmark...");

// Warmup
for(let i=0; i<100; i++) Test.decode(buffer);

// 1. Decode Only (Should be instant)
const startDecode = process.hrtime.bigint();
let lastMsg;
for (let i = 0; i < ITERATIONS; i++) {
    lastMsg = Test.decode(buffer);
}
const endDecode = process.hrtime.bigint();
const decodeTime = Number(endDecode - startDecode) / 1e6;
console.log(`Decode Only: ${decodeTime.toFixed(2)}ms, ${(ITERATIONS/decodeTime*1000).toFixed(0)} ops/sec`);

// 2. Decode + Access One Field
const startAccessOne = process.hrtime.bigint();
for (let i = 0; i < ITERATIONS; i++) {
    const msg = Test.decode(buffer);
    const val = msg.name;
}
const endAccessOne = process.hrtime.bigint();
const accessOneTime = Number(endAccessOne - startAccessOne) / 1e6;
console.log(`Decode + Access 'name': ${accessOneTime.toFixed(2)}ms, ${(ITERATIONS/accessOneTime*1000).toFixed(0)} ops/sec`);

// 3. Decode + Access All Fields
const startAccessAll = process.hrtime.bigint();
for (let i = 0; i < ITERATIONS; i++) {
    const msg = Test.decode(buffer);
    const a = msg.name;
    const b = msg.id;
    const c = msg.score;
    const d = msg.values; // This triggers array decode
    const e = msg.nested; // This triggers nested proxy creation
    // const f = msg.nested.title; // Access nested field
}
const endAccessAll = process.hrtime.bigint();
const accessAllTime = Number(endAccessAll - startAccessAll) / 1e6;
console.log(`Decode + Access All: ${accessAllTime.toFixed(2)}ms, ${(ITERATIONS/accessAllTime*1000).toFixed(0)} ops/sec`);

// 4. JSON Stringify (Full Decode)
const startJson = process.hrtime.bigint();
for (let i = 0; i < ITERATIONS; i++) {
    const msg = Test.decode(buffer);
    JSON.stringify(msg);
}
const endJson = process.hrtime.bigint();
const jsonTime = Number(endJson - startJson) / 1e6;
console.log(`JSON Stringify: ${jsonTime.toFixed(2)}ms, ${(ITERATIONS/jsonTime*1000).toFixed(0)} ops/sec`);

// Verify correctness
console.log("\nVerification:");
const msg = Test.decode(buffer);
console.log("name:", msg.name);
console.log("id:", msg.id);
console.log("values length:", msg.values.length);
console.log("nested title:", msg.nested.title);
console.log("JSON:", JSON.stringify(msg));
