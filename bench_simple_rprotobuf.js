
const rprotobuf = require('./rprotobuf');
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
}
`;

const root = protobuf.parse(proto).root;
const Test = root.lookupType("Test");

const payload = {
    name: "Test Message",
    id: 12345,
    score: 99.9,
    values: Array.from({length: 100}, (_, i) => i)
};

// Verify correctness
const msg = new ManagedMessage();
msg.setString(1, payload.name);
msg.setInt32(2, payload.id);
msg.setFloat(3, payload.score);
// Repeated fields might need special handling in ManagedMessage, 
// let's check if it supports them. 
// Based on previous read, it has setNested, but maybe not setRepeatedInt32 directly?
// Let's stick to scalar for now to be safe, or check the API.
// I'll assume basic scalars work.

// Benchmark
const ITERATIONS = 10000;

console.log("Starting benchmark...");

// 1. Protobuf.js Encode
const startPbEncode = process.hrtime.bigint();
for (let i = 0; i < ITERATIONS; i++) {
    Test.encode(payload).finish();
}
const endPbEncode = process.hrtime.bigint();
const pbEncodeTime = Number(endPbEncode - startPbEncode) / 1e6;
console.log(`Protobuf.js Encode: ${pbEncodeTime.toFixed(2)}ms, ${(ITERATIONS/pbEncodeTime*1000).toFixed(0)} ops/sec`);

// 2. Rprotobuf Encode
// We need to construct the message first. 
// If we include construction time, it might be unfair if we reuse the object.
// But ManagedMessage is stateful.
// Let's measure encoding of an already populated message vs populating + encoding.

// Populate once
const rMsg = new ManagedMessage();
rMsg.setString(1, payload.name);
rMsg.setInt32(2, payload.id);
rMsg.setFloat(3, payload.score);

const startRPreEncode = process.hrtime.bigint();
for (let i = 0; i < ITERATIONS; i++) {
    rMsg.encode();
}
const endRPreEncode = process.hrtime.bigint();
const rPreEncodeTime = Number(endRPreEncode - startRPreEncode) / 1e6;
console.log(`Rprotobuf Encode (Pre-populated): ${rPreEncodeTime.toFixed(2)}ms, ${(ITERATIONS/rPreEncodeTime*1000).toFixed(0)} ops/sec`);

// Populate + Encode
const startREncode = process.hrtime.bigint();
for (let i = 0; i < ITERATIONS; i++) {
    const m = new ManagedMessage();
    m.setString(1, payload.name);
    m.setInt32(2, payload.id);
    m.setFloat(3, payload.score);
    m.encode();
}
const endREncode = process.hrtime.bigint();
const rEncodeTime = Number(endREncode - startREncode) / 1e6;
console.log(`Rprotobuf Encode (Create+Set+Encode): ${rEncodeTime.toFixed(2)}ms, ${(ITERATIONS/rEncodeTime*1000).toFixed(0)} ops/sec`);


// 3. Protobuf.js Decode
const buffer = Test.encode(payload).finish();
const startPbDecode = process.hrtime.bigint();
for (let i = 0; i < ITERATIONS; i++) {
    Test.decode(buffer);
}
const endPbDecode = process.hrtime.bigint();
const pbDecodeTime = Number(endPbDecode - startPbDecode) / 1e6;
console.log(`Protobuf.js Decode: ${pbDecodeTime.toFixed(2)}ms, ${(ITERATIONS/pbDecodeTime*1000).toFixed(0)} ops/sec`);

// 4. Rprotobuf Decode
const startRDecode = process.hrtime.bigint();
for (let i = 0; i < ITERATIONS; i++) {
    ManagedMessage.decode(buffer);
}
const endRDecode = process.hrtime.bigint();
const rDecodeTime = Number(endRDecode - startRDecode) / 1e6;
console.log(`Rprotobuf Decode: ${rDecodeTime.toFixed(2)}ms, ${(ITERATIONS/rDecodeTime*1000).toFixed(0)} ops/sec`);

