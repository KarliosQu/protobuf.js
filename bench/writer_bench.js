"use strict";

var newSuite = require("./suite"),
    payload = require("./data/bench.json");

var protobuf = require("..");
var rprotobuf = require("../../rprotobuf");

// Setup protobuf.js
var pbjsCls = protobuf.loadSync(require.resolve("./data/bench.proto")).resolveAll().lookup("Test");
var pbjsMsg = payload;

// Setup Rust Writer
var { Writer, NativeType } = rprotobuf;

// Setup NativeType Schema
var innerInnerType = new NativeType();
innerInnerType.addField("long", 1, "int64", false);
innerInnerType.addField("enum", 2, "uint32", false);
innerInnerType.addField("sint32", 3, "sint32", false);

var outerType = new NativeType();
outerType.addField("bool", 1, "bool", true); // repeated
outerType.addField("double", 2, "double", false);

var innerType = new NativeType();
innerType.addField("int32", 1, "int32", false);
innerType.addField("inner", 2, "message", false, innerInnerType);
innerType.addField("outer", 3, "message", false, outerType);

var rootType = new NativeType();
rootType.addField("string", 1, "string", false);
rootType.addField("uint32", 2, "uint32", false);
rootType.addField("inner", 3, "message", false, innerType);
rootType.addField("float", 4, "float", false);

function encodeTestMessage(msg, writer) {
    if (msg.string) {
        writer.uint32((1 << 3) | 2); // Tag
        writer.string(msg.string);
    }
    if (msg.uint32) {
        writer.uint32((2 << 3) | 0); // Tag
        writer.uint32(msg.uint32);
    }
    if (msg.inner) {
        writer.uint32((3 << 3) | 2); // Tag
        writer.fork();
        encodeInner(msg.inner, writer);
        writer.ldelim();
    }
    if (msg.float) {
        writer.uint32((4 << 3) | 5); // Tag
        writer.float(msg.float);
    }
}

function encodeInner(msg, writer) {
    if (msg.int32) {
        writer.uint32((1 << 3) | 0);
        writer.int32(msg.int32);
    }
    if (msg.inner) {
        writer.uint32((2 << 3) | 2);
        writer.fork();
        encodeInnerInner(msg.inner, writer);
        writer.ldelim();
    }
    if (msg.outer) {
        writer.uint32((3 << 3) | 2);
        writer.fork();
        encodeOuter(msg.outer, writer);
        writer.ldelim();
    }
}

function encodeInnerInner(msg, writer) {
    if (msg.long) {
        writer.uint32((1 << 3) | 0);
        writer.int64(msg.long);
    }
    if (msg.enum) {
        writer.uint32((2 << 3) | 0);
        writer.uint32(msg.enum);
    }
    if (msg.sint32) {
        writer.uint32((3 << 3) | 0);
        writer.sint32(msg.sint32);
    }
}

function encodeOuter(msg, writer) {
    if (msg.bool) {
        for (var i = 0; i < msg.bool.length; ++i) {
            writer.uint32((1 << 3) | 0);
            writer.bool(msg.bool[i]);
        }
    }
    if (msg.double) {
        writer.uint32((2 << 3) | 1);
        writer.double(msg.double);
    }
}

newSuite("encoding")
    .add("protobuf.js (Standard)", function() {
        pbjsCls.encode(pbjsMsg).finish();
    })
    .add("Rust Writer (Basic)", function() {
        var writer = new Writer();
        encodeTestMessage(pbjsMsg, writer);
        writer.finish();
    })
    .add("Rust NativeType (Schema)", function() {
        rootType.encode(pbjsMsg);
    })
    .run();
