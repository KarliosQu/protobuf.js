var rprotobuf = require("./rprotobuf");
var { ManagedMessage } = rprotobuf;

try {
    var msg = new ManagedMessage();
    
    // Test BigInt
    var bigVal = 1234567890123456789n;
    msg.setInt64(1, bigVal);
    
    var retrieved = msg.getInt64(1);
    console.log("Set BigInt:", bigVal);
    console.log("Get BigInt:", retrieved);
    console.log("Match:", bigVal === retrieved);

    // Test Uint64 (same underlying storage, but API check)
    var uBigVal = 18446744073709551615n; // Max Uint64
    msg.setUint64(2, uBigVal);
    var uRetrieved = msg.getUint64(2);
    console.log("Set Uint64:", uBigVal);
    console.log("Get Uint64:", uRetrieved);
    // Note: BigInt in JS is signed, so max uint64 might show as -1 if treated as signed int64 by NAPI BigInt::from(u64)
    // Let's see how NAPI handles it.
    
    console.log("Success!");
} catch (e) {
    console.error("Error:", e);
}
