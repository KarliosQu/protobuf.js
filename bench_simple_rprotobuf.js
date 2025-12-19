
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
    Nested nested = 5;
    repeated float scores = 6;
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
    scores: Array.from({length: 100}, (_, i) => i * 0.5)
};

// Verify correctness
const msg = new ManagedMessage();
msg.setString(1, payload.name);
msg.setInt32(2, payload.id);
msg.setFloat(3, payload.score);
msg.setPackedInt32(4, payload.values);
msg.setPackedFloat(6, payload.scores);
const nestedMsg = new ManagedMessage();
nestedMsg.setString(1, "Nested Title");
msg.setNested(5, nestedMsg);


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
rMsg.setPackedInt32(4, payload.values);
rMsg.setPackedFloat(6, payload.scores);
const nestedMsg2 = new ManagedMessage();
nestedMsg2.setString(1, "Nested Title");
rMsg.setNested(5, nestedMsg2);

const startRPreEncode = process.hrtime.bigint();
for (let i = 0; i < ITERATIONS; i++) {
    const b = rMsg.encode();
    if (i===0) {
        console.log("Rprotobuf encoded buffer length:", b.length);
        console.log("Rprotobuf encoded buffer hex:", b.subarray(0, 20).toString('hex'));
        const d = ManagedMessage.decode(b);
        console.log("Loopback decode has(1)?", d.has(1));
        console.log("Loopback decode has(4)?", d.has(4));
        console.log("Loopback decode has(6)?", d.has(6));
    }
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
// Use Rprotobuf encoded buffer if Protobuf.js one is suspicious, or just use Rprotobuf one to be safe for Rprotobuf test.
const buffer = rMsg.encode(); 
console.log("Buffer length:", buffer.length);
console.log("Buffer hex:", buffer.subarray(0, 20).toString('hex'));

const startPbDecode = process.hrtime.bigint();
for (let i = 0; i < ITERATIONS; i++) {
    Test.decode(buffer);
}
const endPbDecode = process.hrtime.bigint();
const pbDecodeTime = Number(endPbDecode - startPbDecode) / 1e6;
console.log(`Protobuf.js Decode: ${pbDecodeTime.toFixed(2)}ms, ${(ITERATIONS/pbDecodeTime*1000).toFixed(0)} ops/sec`);

// 4. Rprotobuf Decode
const startRDecode = process.hrtime.bigint();
let decodedMsg;
for (let i = 0; i < ITERATIONS; i++) {
    decodedMsg = ManagedMessage.decode(buffer);
}
const endRDecode = process.hrtime.bigint();
const rDecodeTime = Number(endRDecode - startRDecode) / 1e6;
console.log(`Rprotobuf Decode: ${rDecodeTime.toFixed(2)}ms, ${(ITERATIONS/rDecodeTime*1000).toFixed(0)} ops/sec`);

console.log("Decoded has(1)?", decodedMsg.has(1));
console.log("Decoded has(4)?", decodedMsg.has(4));
console.log("Decoded has(6)?", decodedMsg.has(6));
const decodedValues = decodedMsg.getPackedInt32(4);
console.log("Decoded values type:", decodedValues ? decodedValues.constructor.name : "null");
console.log("Decoded values length:", decodedValues ? decodedValues.length : 0);
console.log("Decoded values[0]:", decodedValues ? decodedValues[0] : "N/A");

const decodedScores = decodedMsg.getPackedFloat(6);
console.log("Decoded scores type:", decodedScores ? decodedScores.constructor.name : "null");
console.log("Decoded scores length:", decodedScores ? decodedScores.length : 0);
console.log("Decoded scores[0]:", decodedScores ? decodedScores[0] : "N/A");

