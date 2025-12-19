
const rprotobuf = require('./rprotobuf');
const { ManagedMessage } = rprotobuf;
const protobuf = require('.');
const payload = require('./bench/data/bench.json');

const root = protobuf.loadSync(require.resolve('./bench/data/bench.proto'));
const Test = root.lookupType("Test");
const pbjsBuf = Test.encode(payload).finish();

console.log("Buffer length:", pbjsBuf.length);

try {
    const msg = ManagedMessage.decode(pbjsBuf);
    console.log("Decode success");
} catch (e) {
    console.error("Decode failed:", e);
}
