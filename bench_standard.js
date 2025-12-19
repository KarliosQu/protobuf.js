const protobuf = require('./src/index'); // Standard protobuf.js
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
console.log("Benchmarking Standard Encode...");
while (Date.now() - startEncode < duration) {
    Test.encode(payload).finish();
    countEncode++;
}
const opsEncode = countEncode / (duration / 1000);
console.log(`Encode Ops/sec: ${opsEncode.toFixed(2)}`);

