"use strict";

var newSuite = require("./suite");
var protobuf = require("..");
var rprotobuf = require("../../rprotobuf");
var { NativeMessage, NativeType, Writer, ManagedMessage } = rprotobuf;

// --- Setup Schemas ---

// 1. Protobuf.js Setup
var root = protobuf.loadSync(require.resolve("./data/bench.proto")).resolveAll();
var Test = root.lookupType("Test");

// 2. NativeType Setup (Manual Schema Definition)
var innerInnerType = new NativeType();
innerInnerType.addField("long", 1, "int64", false, null);
innerInnerType.addField("enum", 2, "uint32", false, null);
innerInnerType.addField("sint32", 3, "sint32", false, null);

var outerType = new NativeType();
outerType.addField("bool", 1, "bool", true, null); // repeated
outerType.addField("double", 2, "double", false, null);

var innerType = new NativeType();
innerType.addField("int32", 1, "int32", false, null);
innerType.addField("innerInner", 2, "message", false, innerInnerType);
innerType.addField("outer", 3, "message", false, outerType);

var rootType = new NativeType();
rootType.addField("string", 1, "string", false, null);
rootType.addField("uint32", 2, "uint32", false, null);
rootType.addField("inner", 3, "message", false, innerType);
rootType.addField("float", 4, "float", false, null);

// 3. ManagedMessage Setup (Helper to populate data)
function createManagedMessage(data) {
    var msg = new ManagedMessage();
    msg.setString(1, data.string);
    msg.setUint32(2, data.uint32);
    msg.setFloat(4, data.float);
    
    var inner = new ManagedMessage();
    inner.setInt32(1, data.inner.int32);
    
    var innerInner = new ManagedMessage();
    // innerInner.setInt64(1, data.inner.innerInner.long); // Need BigInt support or cast
    innerInner.setUint32(2, data.inner.innerInner.enum);
    innerInner.setInt32(3, data.inner.innerInner.sint32);
    inner.setNested(2, innerInner);
    
    var outer = new ManagedMessage();
    outer.setDouble(2, data.inner.outer.double);
    // Repeated bool not implemented in ManagedMessage demo yet
    inner.setNested(3, outer);
    
    msg.setNested(3, inner);
    return msg;
}

// --- Data Generation ---

// --- Data Generation ---

function generateData(sizeMultiplier) {
    var strLen = 20 * sizeMultiplier;
    var arrLen = 10 * sizeMultiplier;
    
    return {
        string: "a".repeat(strLen),
        uint32: 123456,
        inner: {
            int32: -123,
            innerInner: {
                long: 9876543210,
                enum: 2,
                sint32: -456
            },
            outer: {
                bool: Array(arrLen).fill(true),
                double: 123.456
            }
        },
        float: 789.123
    };
}

var smallData = generateData(1);
var mediumData = generateData(100); // Longer strings, larger arrays
var largeData = generateData(1000); // Huge strings, huge arrays

// Pre-encode buffers for decoding tests
var smallBuf = Test.encode(smallData).finish();
var mediumBuf = Test.encode(mediumData).finish();
var largeBuf = Test.encode(largeData).finish();

console.log(`Buffer Sizes - Small: ${smallBuf.length}b, Medium: ${mediumBuf.length}b, Large: ${largeBuf.length}b`);

// --- Benchmarks ---

// 1. Decoding Performance
newSuite("Decoding (Small)")
    .add("protobuf.js", () => Test.decode(smallBuf))
    .add("Rust NativeMessage", () => {
        var msg = new NativeMessage();
        msg.decode(smallBuf);
    })
    .run();

newSuite("Decoding (Medium)")
    .add("protobuf.js", () => Test.decode(mediumBuf))
    .add("Rust NativeMessage", () => {
        var msg = new NativeMessage();
        msg.decode(mediumBuf);
    })
    .run();

newSuite("Decoding (Large)")
    .add("protobuf.js", () => Test.decode(largeBuf))
    .add("Rust NativeMessage", () => {
        var msg = new NativeMessage();
        msg.decode(largeBuf);
    })
    .run();

// 2. Encoding Performance
newSuite("Encoding (Small)")
    .add("protobuf.js", () => Test.encode(smallData).finish())
    .add("Rust NativeType", () => rootType.encode(smallData))
    .run();

newSuite("Encoding (Medium)")
    .add("protobuf.js", () => Test.encode(mediumData).finish())
    .add("Rust NativeType", () => rootType.encode(mediumData))
    .run();

newSuite("Encoding (Large)")
    .add("protobuf.js", () => Test.encode(largeData).finish())
    .add("Rust NativeType", () => rootType.encode(largeData))
    .add("Rust ManagedMessage", () => {
        managedLarge.encode();
    })
    .run();

// 3. Field Access Performance (Lazy vs Eager)
// For this, we decode once, then access fields repeatedly
var jsMsgSmall = Test.decode(smallBuf);
var rustMsgSmall = new NativeMessage(); rustMsgSmall.decode(smallBuf);
var managedSmall = createManagedMessage(smallData);

var jsMsgLarge = Test.decode(largeBuf);
var rustMsgLarge = new NativeMessage(); rustMsgLarge.decode(largeBuf);
var managedLarge = createManagedMessage(largeData);

newSuite("Field Access (Small - String)")
    .add("JS Object", () => { var s = jsMsgSmall.string; })
    .add("Rust Native", () => { var s = rustMsgSmall.getString(1); })
    .add("Rust Managed", () => { var s = managedSmall.getString(1); })
    .run();

newSuite("Field Access (Large - String)")
    .add("JS Object", () => { var s = jsMsgLarge.string; })
    .add("Rust Native", () => { var s = rustMsgLarge.getString(1); })
    .add("Rust Managed", () => { var s = managedLarge.getString(1); })
    .run();

newSuite("Field Access (Large - Repeated Bool)")
    .add("JS Object", () => { var len = jsMsgLarge.inner.outer.bool.length; })
    .add("Rust Native", () => { 
        // Accessing nested repeated field
        var inner = rustMsgLarge.getMessage(3);
        if (inner) {
            var outer = inner.getMessage(3);
            if (outer) {
                var len = outer.getCount(1);
            }
        }
    })
    .run();
