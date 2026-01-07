const protobuf = require("../index");
const RustModule = require("./index");

/**
 * Enable the Rust optimization layers.
 * Currently replaces Writer and Reader with Native implementations.
 */
function enable() {
    if (!RustModule.Writer || !RustModule.Reader) {
        console.warn("Protobuf Rust extension loaded but symbols missing.");
        return;
    }

    // --- Writer Replacement ---
    const OriginalWriter = protobuf.Writer;
    
    // We create a wrapper to adapt the N-API Writer to protobuf.js Writer interface
    // protobuf.js Writer is a class that users instantiate (Writer.create()) or use new Writer()
    
    // The Rust Writer matches most of the API, but we need to ensure static methods exist.
    
    // We can't easily "extend" the Rust class in JS and keep the native speed for methods, 
    // so we patch the prototype or wrapper.
    
    // For now, simpler approach: Wrappers.
    // Note: This is an I/O layer optimization integration.
    
    // In mirror, they used ManagedMessage. Here we replace the low-level primitives.
    
    // To properly integrating without ManagedMessage, we would need to patch usage sites.
    // For this stage, we simply expose them on the util to be used manually or by advanced users,
    // OR we attempt a prototype injection.

    // Let's attach to util for explicit usage first, as strict replacement might break existing JS logic 
    // that relies on internal state (underscore properties).
    protobuf.util.RustWriter = RustModule.Writer;
    protobuf.util.RustReader = RustModule.Reader;
    
    // Attempt global replacement if requested
    if (process.env.PROTOBUF_REPLACE_IO) {
        console.log("Replacing Protobuf IO with Rust Native implementation...");
        protobuf.Writer = RustModule.Writer;
        // Re-attach static methods that might be needed
        protobuf.Writer.create = RustModule.Writer.create || function() { return new RustModule.Writer(); };
        
        protobuf.Reader = RustModule.Reader;
        protobuf.Reader.create = RustModule.Reader.create || function(buf) { return new RustModule.Reader(buf); };
    }
}

module.exports = { enable };
