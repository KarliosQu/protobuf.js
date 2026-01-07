process.env.PROTOBUF_USE_RUST = 'true';
const protobuf = require('./index');

console.log("Testing Rust Integration...");
try {
    const w = protobuf.Writer.create();
    // Test basic writer
    w.uint32(100);
    w.string("Hello Rust");
    const buf = w.finish();
    console.log(`Buffer created (len=${buf.length}):`, buf);
    
    // Test basic reader
    const r = protobuf.Reader.create(buf);
    const num = r.uint32();
    const str = r.string();
    
    console.log(`Read back: num=${num}, str="${str}"`);
    
    if (num === 100 && str === "Hello Rust") {
        console.log("SUCCESS: Read/Write match!");
    } else {
        console.error("FAILURE: Mismatch");
        process.exit(1);
    }
} catch(e) {
    console.error("CRITICAL FAILURE:", e);
    process.exit(1);
}
