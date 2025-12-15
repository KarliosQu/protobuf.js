var tape = require("tape");
var nativeBridge = require("../src/util/native_bridge");

tape.test("native bridge utilities", function(test) {
    
    console.log("Using: " + nativeBridge.getNativeInfo());
    
    test.test("ucFirst", function(test) {
        test.equal(nativeBridge.ucFirst("hello"), "Hello", "should capitalize first letter");
        test.equal(nativeBridge.ucFirst("Hello"), "Hello", "should handle already capitalized");
        test.equal(nativeBridge.ucFirst(""), "", "should handle empty string");
        test.equal(nativeBridge.ucFirst("a"), "A", "should handle single character");
        test.end();
    });

    test.test("camelCase", function(test) {
        test.equal(nativeBridge.camelCase("hello_world"), "helloWorld", "should convert snake_case to camelCase");
        test.equal(nativeBridge.camelCase("hello_world_test"), "helloWorldTest", "should handle multiple underscores");
        test.equal(nativeBridge.camelCase("hello"), "hello", "should handle no underscores");
        test.equal(nativeBridge.camelCase("_hello"), "_hello", "should keep leading underscore");
        test.end();
    });

    test.test("isReserved", function(test) {
        // Test reserved keywords
        test.equal(nativeBridge.isReserved("if"), true, "should recognize 'if' as reserved");
        test.equal(nativeBridge.isReserved("for"), true, "should recognize 'for' as reserved");
        test.equal(nativeBridge.isReserved("class"), true, "should recognize 'class' as reserved");
        test.equal(nativeBridge.isReserved("return"), true, "should recognize 'return' as reserved");
        test.equal(nativeBridge.isReserved("function"), true, "should recognize 'function' as reserved");
        
        // Test non-reserved names
        test.equal(nativeBridge.isReserved("myVar"), false, "should not recognize 'myVar' as reserved");
        test.equal(nativeBridge.isReserved("hello"), false, "should not recognize 'hello' as reserved");
        test.equal(nativeBridge.isReserved("test123"), false, "should not recognize 'test123' as reserved");
        
        test.end();
    });

    test.test("safeProp", function(test) {
        test.equal(nativeBridge.safeProp("myProp"), ".myProp", "should return dot notation for safe prop");
        test.equal(nativeBridge.safeProp("my_prop"), ".my_prop", "should handle underscores");
        test.equal(nativeBridge.safeProp("class"), "[\"class\"]", "should use bracket notation for reserved word");
        test.equal(nativeBridge.safeProp("if"), "[\"if\"]", "should use bracket notation for 'if'");
        test.equal(nativeBridge.safeProp("my-prop"), "[\"my-prop\"]", "should use bracket notation for hyphens");
        test.equal(nativeBridge.safeProp("my prop"), "[\"my prop\"]", "should use bracket notation for spaces");
        test.end();
    });

    test.test("toArray", function(test) {
        var obj = { a: 1, b: 2, c: 3 };
        var arr = nativeBridge.toArray(obj);
        test.equal(arr.length, 3, "should convert object to array of correct length");
        test.ok(arr.includes(1), "should include value 1");
        test.ok(arr.includes(2), "should include value 2");
        test.ok(arr.includes(3), "should include value 3");
        
        test.deepEqual(nativeBridge.toArray({}), [], "should handle empty object");
        test.deepEqual(nativeBridge.toArray(null), [], "should handle null");
        test.deepEqual(nativeBridge.toArray(undefined), [], "should handle undefined");
        
        test.end();
    });

    test.test("toObject", function(test) {
        var arr = ["key1", "value1", "key2", "value2", "key3", "value3"];
        var obj = nativeBridge.toObject(arr);
        test.equal(obj.key1, "value1", "should convert array to object correctly");
        test.equal(obj.key2, "value2", "should handle multiple key-value pairs");
        test.equal(obj.key3, "value3", "should handle all pairs");
        
        // Test with undefined values (should be omitted)
        var arr2 = ["key1", "value1", "key2", undefined, "key3", "value3"];
        var obj2 = nativeBridge.toObject(arr2);
        test.equal(obj2.key1, "value1", "should include defined values");
        test.ok(!obj2.hasOwnProperty("key2"), "should omit undefined values");
        test.equal(obj2.key3, "value3", "should continue after undefined");
        
        test.deepEqual(nativeBridge.toObject([]), {}, "should handle empty array");
        
        test.end();
    });

    test.test("native info", function(test) {
        var info = nativeBridge.getNativeInfo();
        test.ok(typeof info === "string", "should return string info");
        test.ok(info.length > 0, "should return non-empty info");
        console.log("Native bridge info:", info);
        test.end();
    });

    test.end();
});
