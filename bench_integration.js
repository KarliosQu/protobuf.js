const protobuf = require('./rprotobuf/integration'); // This patches protobuf.js
const fs = require('fs');
const path = require('path');

const root = protobuf.loadSync(path.join(__dirname, 'bench/data/bench.proto'));
const Test = root.lookupType("Test");
const payload = require('./bench/data/bench.json');

// Encode once to get buffer
const buf = Test.encode(payload).finish();
console.log("Buffer size:", buf.length);

const duration = 2000;
// Benchmark Encode
const startEncode = Date.now();
let countEncode = 0;
console.log("Benchmarking Drop-in Encode (Rust-backed)...");
while (Date.now() - startEncode < duration) {
    Test.encode(payload).finish();
    countEncode++;
}
const opsEncode = countEncode / (duration / 1000);
console.log(`Encode Ops/sec: ${opsEncode.toFixed(2)}`);

// Benchmark Decode
const startDecode = Date.now();
let countDecode = 0;
console.log("Benchmarking Drop-in Decode (Rust-backed)...");
while (Date.now() - startDecode < duration) {
    Test.decode(buf);
    countDecode++;
}
const opsDecode = countDecode / (duration / 1000);
console.log(`Decode Ops/sec: ${opsDecode.toFixed(2)}`);

// Verify correctness
const decoded = Test.decode(buf);
console.log("Decoded string:", decoded.string);
console.log("Decoded uint32:", decoded.uint32);
console.log("Decoded inner.int32:", decoded.inner.int32);

