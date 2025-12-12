var protobuf = require("..");

try {
    var root = protobuf.loadSync("tests/data/convert.proto");
    var Message = root.lookup("Message");

    console.log("Testing Reader with empty buffer...");
    var Reader = protobuf.Reader;
    try {
        var r = Reader.create(Buffer.alloc(0));
        console.log("Reader created with empty buffer");
    } catch (e) {
        console.log("Reader failed:", e);
    }

    console.log("Creating message...");
    var msg = Message.create();
    console.log("Message created.");

    console.log("Converting to object with defaults...");
    var obj = Message.toObject(msg, { defaults: true });
    console.log("Converted:", obj);
} catch (e) {
    console.error("Error:", e);
}

console.log("Testing Reader with empty buffer...");
var Reader = protobuf.Reader;
try {
    var r = Reader.create(Buffer.alloc(0));
    console.log("Reader created with empty buffer");
} catch (e) {
    console.log("Reader failed:", e);
}
