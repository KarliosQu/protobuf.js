const { fromObject, toObject } = require('./adapter');
const { ManagedMessage } = require('./index');
const protobuf = require('..'); // Load protobuf.js from parent

// Define a simple proto
const proto = `
syntax = "proto3";

message TestMessage {
    int32 id = 1;
    string name = 2;
    repeated int32 values = 3;
    repeated int64 big_values = 4;
    repeated int32 packed_values = 5 [packed=true];
}
`;

const root = protobuf.parse(proto).root;
const TestMessage = root.lookupType("TestMessage");
console.log("Fields:", TestMessage.fieldsArray.map(f => f.name));

// Test fromObject
const obj = {
    id: 123,
    name: "Hello",
    values: [1, 2, 3],
    bigValues: [100n, 200n], // BigInts
    packedValues: [10, 20, 30]
};

console.log("Original Object:", obj);

const msg = fromObject(TestMessage, obj);
console.log("ManagedMessage created.");

// Test toObject
const obj2 = toObject(TestMessage, msg, { longs: String });
console.log("Converted back Object:", obj2);

// Verify
if (obj2.id !== obj.id) console.error("ID mismatch");
if (obj2.name !== obj.name) console.error("Name mismatch");
if (JSON.stringify(obj2.values) !== JSON.stringify(obj.values)) console.error("Values mismatch");
if (obj2.bigValues[0] !== "100") console.error("BigValues mismatch");
if (JSON.stringify(obj2.packedValues) !== JSON.stringify(obj.packedValues)) console.error("PackedValues mismatch");

console.log("Test finished.");
