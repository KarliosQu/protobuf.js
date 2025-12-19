import protobuf from "protobufjs/minimal";

// Minimal build does not include reflection (Type, Root, etc.), so we cannot patch Type.prototype.encode/decode.
// It is used for static code generation where Writer/Reader are used directly.
// Therefore, no Rust acceleration is applied here.

export default protobuf;

export const {
    Message,
    Reader, Writer,
    util, configure,
    common, converter, decoder, encoder, verifier, wrappers,
    types
} = protobuf;
