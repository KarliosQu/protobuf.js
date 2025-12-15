const native = require('./index');

class Writer {
    constructor() {
        this.native = new native.Writer();
    }

    static create() {
        return new Writer();
    }

    uint32(value) {
        this.native.uint32(value);
        return this;
    }

    int32(value) {
        this.native.int32(value);
        return this;
    }

    sint32(value) {
        this.native.sint32(value);
        return this;
    }

    bool(value) {
        this.native.bool(value);
        return this;
    }

    fixed32(value) {
        this.native.fixed32(value);
        return this;
    }

    sfixed32(value) {
        this.native.sfixed32(value);
        return this;
    }

    float(value) {
        this.native.float(value);
        return this;
    }

    double(value) {
        this.native.double(value);
        return this;
    }

    string(value) {
        this.native.string(value);
        return this;
    }

    bytes(value) {
        this.native.bytes(value);
        return this;
    }

    raw(value) {
        this.native.raw(value);
        return this;
    }

    fork() {
        this.native.fork();
        return this;
    }

    ldelim() {
        this.native.ldelim();
        return this;
    }

    reset() {
        this.native.reset();
        return this;
    }

    finish() {
        return this.native.finish();
    }

    get len() {
        return this.native.len;
    }

    uint64(value) {
        this.native.uint64(value);
        return this;
    }

    int64(value) {
        this.native.int64(value);
        return this;
    }

    sint64(value) {
        this.native.sint64(value);
        return this;
    }

    fixed64(value) {
        this.native.fixed64(value);
        return this;
    }

    sfixed64(value) {
        this.native.sfixed64(value);
        return this;
    }
}

module.exports = {
    ...native,
    Writer
};