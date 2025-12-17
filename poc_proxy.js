const protobuf = require('./src/index');
const { ManagedMessage } = require('./rprotobuf/index');

// 1. Load definition
const root = protobuf.loadSync("bench/data/bench.proto");
const TestType = root.lookupType("Test");

// 2. Factory function to create a "Transparent" ManagedMessage
function createTransparentMessage(type, properties) {
    const managed = new ManagedMessage();
    
    // Helper to map field names to IDs and Types
    const fieldMap = {};
    for(const field of type.fieldsArray) {
        fieldMap[field.name] = { id: field.id, type: field.type };
    }

    // Initialize with properties if provided
    if (properties) {
        for (const key in properties) {
            const info = fieldMap[key];
            if (info) {
                // Simplified: only handling string/uint32 for demo
                if (info.type === "string") managed.setString(info.id, properties[key]);
                else if (info.type === "uint32") managed.setUint32(info.id, properties[key]);
            }
        }
    }

    // 3. Create Proxy
    return new Proxy({}, {
        get(target, prop) {
            if (prop === "$managed") return managed; // Backdoor to access underlying Rust object
            
            const info = fieldMap[prop];
            if (!info) return target[prop];

            // Read from Rust
            if (info.type === "string") return managed.getString(info.id);
            if (info.type === "uint32") return managed.getUint32(info.id);
            return undefined;
        },
        set(target, prop, value) {
            const info = fieldMap[prop];
            if (!info) {
                target[prop] = value;
                return true;
            }

            // Write to Rust
            if (info.type === "string") managed.setString(info.id, value);
            else if (info.type === "uint32") managed.setUint32(info.id, value);
            
            return true;
        }
    });
}

// --- Usage Demo ---

console.log("Creating transparent message...");
const msg = createTransparentMessage(TestType, {
    string: "Hello Rust",
    uint32: 12345
});

console.log("Reading properties (from Rust):");
console.log("msg.string:", msg.string);
console.log("msg.uint32:", msg.uint32);

console.log("\nModifying property...");
msg.string = "Modified Value";
console.log("msg.string:", msg.string);

console.log("\nUnderlying Rust object state:");
// We can verify the Rust object was updated by encoding it
const buf = msg.$managed.encode();
console.log("Encoded buffer length:", buf.length);

// Verify we can decode it back using standard protobuf.js
const decoded = TestType.decode(buf);
console.log("Decoded with standard lib:", decoded);
