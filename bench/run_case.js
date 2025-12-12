const Benchmark = require("benchmark");
const path = require("path");

const mode = process.argv[2];
const targetSize = parseInt(process.argv[3]);

// Load Protobuf
let protobuf;
try {
    if (mode === 'native') {
        protobuf = require("../../rprotobuf/integration_native.js");
    } else {
        protobuf = require("../../rprotobuf/integration.js");
    }
} catch (e) {
    console.error("Failed to load protobuf integration:", e);
    process.exit(1);
}

// Load Proto
const root = protobuf.loadSync(path.join(__dirname, "bench_multi.proto"));
const BenchMessage = root.lookupType("BenchMessage");

// Generate Data
// We want the final binary size to be close to targetSize.
// Each item has overhead + string + uint32 + bytes.
// Let's assume ~80 bytes per item.
const payloadSize = 50;
const itemOverhead = 30; 
const estItemSize = payloadSize + itemOverhead;
const itemCount = Math.max(1, Math.floor(targetSize / estItemSize));

const payload = {
    items: []
};

for (let i = 0; i < itemCount; i++) {
    payload.items.push({
        id: "item_" + i,
        count: i,
        payload: Buffer.alloc(payloadSize, 1)
    });
}

// Verify Message
const errMsg = BenchMessage.verify(payload);
if (errMsg) throw Error(errMsg);

const message = BenchMessage.create(payload);
const buffer = BenchMessage.encode(message).finish();
const actualSize = buffer.length;


// Verify Decode
try {
    const decoded = BenchMessage.decode(buffer);
    if (!decoded.items || decoded.items.length !== itemCount) {
        // console.error(`[${mode}] Decode mismatch! Expected ${itemCount}, got ${decoded.items ? decoded.items.length : 'undefined'}`);
        // Force failure in stats
        throw new Error("Decode mismatch");
    }
} catch (e) {
    console.error(`[${mode}] Decode error:`, e.message);
    process.exit(1);
}

// console.log(`[${mode}] Target: ${targetSize}, Actual: ${actualSize}, Items: ${itemCount}`);

const suite = new Benchmark.Suite;

suite.add('encode', function() {
    BenchMessage.encode(message).finish();
})
.add('decode', function() {
    BenchMessage.decode(buffer);
})
.on('cycle', function(event) {
    // Output: "encode x 100 ops/sec ..."
    // We want to parse this in the orchestrator.
    // Let's print a JSON object for easier parsing.
    const bench = event.target;
    console.log(JSON.stringify({
        mode: mode,
        size: actualSize,
        type: bench.name,
        ops: bench.hz,
        rme: bench.stats.rme
    }));
})
.run({ 'async': false });
