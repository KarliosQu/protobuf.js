var protobuf = require("./src/index");

var protoRequired = "message Test {\
    required group MyGroup = 1 {\
        option foo = \"bar\";\
        required uint32 a = 2;\
    };\
}";

var root = protobuf.parse(protoRequired).root;
var Test = root.resolveAll().lookup("Test");
var msg = {
    myGroup: {
        a: 111
    }
};

var buf = Test.encode(msg).finish();
var decoded = Test.decode(buf);

console.log("Decoded:", decoded);
console.log("Constructor:", decoded.constructor.name);
if (decoded.myGroup) {
    console.log("MyGroup Constructor:", decoded.myGroup.constructor.name);
    console.log("Is myGroup instance of MyGroup?", decoded.myGroup instanceof Test.get("MyGroup").ctor);
}

var tape = require("tape");
tape.test("debug", function(t) {
    t.same(decoded, msg, "should be same");
    t.end();
});
