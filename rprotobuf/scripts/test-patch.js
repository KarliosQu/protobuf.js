const path = require('path');
// Resolve the path to the original protobufjs package
// rprotobuf is at .../protobuf.js-protobufjs-v7.2.4/rprotobuf
// source is at .../protobuf.js-protobufjs-v7.2.4/protobuf.js-protobufjs-v7.2.4
// scripts is at .../protobuf.js-protobufjs-v7.2.4/rprotobuf/scripts
// So we need to go up 2 levels to get to parent, then down to source.
const protobufPath = path.resolve(__dirname, '../../protobuf.js-protobufjs-v7.2.4');
const rprotobuf = require('../index');

console.log('Patching protobufjs with rprotobuf...');

// Patch module cache for Reader and Writer
// This ensures that internal requires within protobufjs get the patched versions
const readerPath = path.join(protobufPath, 'src/reader.js');
const writerPath = path.join(protobufPath, 'src/writer.js');

// Load them first to populate cache
require(readerPath);
require(writerPath);

// Helper to add static create if missing
if (!rprotobuf.Writer.create) {
    rprotobuf.Writer.create = function create() {
        return new rprotobuf.Writer();
    };
}
if (!rprotobuf.Reader.create) {
    rprotobuf.Reader.create = function create(buf) {
        return new rprotobuf.Reader(buf);
    };
}

// Replace exports in cache
require.cache[require.resolve(readerPath)].exports = rprotobuf.Reader;
require.cache[require.resolve(writerPath)].exports = rprotobuf.Writer;

const protobuf = require(protobufPath);

// Also patch the main protobuf object
protobuf.Writer = rprotobuf.Writer;
protobuf.Reader = rprotobuf.Reader;

// Patch Type.prototype.setup to use our integration logic
// This is crucial because the original setup generates code that uses the original Reader/Writer
// and we want to use our Rust-backed implementation via adapter.js
// const integration = require('../integration');
const integration = require('../integration_native');
protobuf.Type.prototype.setup = integration.Type.prototype.setup;

console.log('Patched protobufjs Writer and Reader via module cache.');

