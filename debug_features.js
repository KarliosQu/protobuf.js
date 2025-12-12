var rprotobuf = require("./rprotobuf");
var { ManagedMessage } = rprotobuf;

try {
    var msg = new ManagedMessage();
    msg.setInt32(1, 100);
    console.log("Has 1:", msg.has(1));
    console.log("Get 1:", msg.getInt32(1));
    
    msg.remove(1);
    console.log("Has 1 after remove:", msg.has(1));
    console.log("Get 1 after remove:", msg.getInt32(1));
    console.log("Get 1 with default:", msg.getInt32WithDefault(1, 999));

    msg.setString(2, "hello");
    console.log("Get 2:", msg.getString(2));
    console.log("Get 2 with default:", msg.getStringWithDefault(2, "world"));
    msg.remove(2);
    console.log("Get 2 with default after remove:", msg.getStringWithDefault(2, "world"));

    console.log("Success!");
} catch (e) {
    console.error("Error:", e);
}
