const protobuf = require("./integration");

// Define a type
const Type = protobuf.Type;
const Field = protobuf.Field;

const Person = new Type("Person");
Person.add(new Field("name", 1, "string"));
Person.add(new Field("age", 2, "int32"));
Person.add(new Field("tags", 3, "string", "repeated"));

// Create a message
const payload = {
    name: "Alice",
    age: 30,
    tags: ["developer", "rust"]
};

console.log("Original payload:", payload);

// Verify
const err = Person.verify(payload);
if (err) throw Error("Verify failed: " + err);
console.log("Verify passed");

// Encode
const buffer = Person.encode(payload).finish();
console.log("Encoded buffer length:", buffer.length);
console.log("Encoded buffer:", buffer);

// Decode
const decoded = Person.decode(buffer);
console.log("Decoded object:", decoded);

// Check equality
if (decoded.name !== payload.name) throw Error("Name mismatch");
if (decoded.age !== payload.age) throw Error("Age mismatch");
if (decoded.tags.length !== payload.tags.length) throw Error("Tags length mismatch");
if (decoded.tags[0] !== payload.tags[0]) throw Error("Tag 0 mismatch");

console.log("Integration test passed!");
