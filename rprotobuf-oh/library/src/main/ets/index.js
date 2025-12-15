import rprotobuf from 'librprotobuf.so';

export const encodeVarint = rprotobuf.encodeVarint;
export const Writer = rprotobuf.Writer;
export const Reader = rprotobuf.Reader;
export const NativeMessage = rprotobuf.NativeMessage;
export const NativeType = rprotobuf.NativeType;
export const ManagedMessage = rprotobuf.ManagedMessage;
export const fast_encoder = rprotobuf.fast_encoder;

export * from "./adapter";
