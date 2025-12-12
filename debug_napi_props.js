
const rust = require("../rprotobuf");

console.log("rust.Writer:", rust.Writer);
console.log("Type of rust.Writer:", typeof rust.Writer);

rust.Writer.foo = "bar";
console.log("rust.Writer.foo:", rust.Writer.foo);

rust.Writer._configure = function() { console.log("configure called"); };
console.log("rust.Writer._configure:", rust.Writer._configure);
