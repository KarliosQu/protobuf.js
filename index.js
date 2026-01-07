// full library entry point.

"use strict";
module.exports = require("./src/index");

// Rust Extension Auto-loader
if (typeof process === 'object' && process && process.env) {
    try {
        if (process.env.PROTOBUF_USE_RUST === 'true' || process.env.PROTOBUF_REPLACE_IO === 'true') {
            // Force replacement if generic enable flag is set
            if (process.env.PROTOBUF_USE_RUST === 'true') {
                process.env.PROTOBUF_REPLACE_IO = 'true';
            }
            require("./rust/integration").enable();
        }
    } catch (e) {
        // Silent failure unless debug is on
        if (process.env.PROTOBUF_DEBUG) {
            console.warn("Protobuf Rust Extension failed to load:", e);
        }
    }
}

