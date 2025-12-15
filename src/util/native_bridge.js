"use strict";

/**
 * Bridge module to integrate Rust native utilities with JavaScript implementation
 * This module attempts to load the native module and falls back to JavaScript if unavailable
 */

let native = null;
let useNative = false;

// Try to load the native module
try {
    native = require('../../native');
    useNative = true;
    // Verify the module loaded correctly by testing a function
    if (typeof native.init !== 'function') {
        useNative = false;
        native = null;
    }
} catch (error) {
    // Native module not available, will use JavaScript fallback
    useNative = false;
}

/**
 * Converts the first character of a string to upper case.
 * @param {string} str String to convert
 * @returns {string} Converted string
 */
exports.ucFirst = function ucFirst(str) {
    if (useNative && native.ucFirst) {
        return native.ucFirst(str);
    }
    return str.charAt(0).toUpperCase() + str.substring(1);
};

var camelCaseRe = /_([a-z])/g;

/**
 * Converts a string to camel case.
 * @param {string} str String to convert
 * @returns {string} Converted string
 */
exports.camelCase = function camelCase(str) {
    if (useNative && native.camelCase) {
        return native.camelCase(str);
    }
    return str.substring(0, 1)
         + str.substring(1)
               .replace(camelCaseRe, function($0, $1) { return $1.toUpperCase(); });
};

/**
 * Tests whether the specified name is a reserved word in JS.
 * @param {string} name Name to test
 * @returns {boolean} `true` if reserved, otherwise `false`
 */
exports.isReserved = function isReserved(name) {
    if (useNative && native.isReserved) {
        return native.isReserved(name);
    }
    return /^(?:do|if|in|for|let|new|try|var|case|else|enum|eval|false|null|this|true|void|with|break|catch|class|const|super|throw|while|yield|delete|export|import|public|return|static|switch|typeof|default|extends|finally|package|private|continue|debugger|function|arguments|interface|protected|implements|instanceof)$/.test(name);
};

var safePropBackslashRe = /\\/g,
    safePropQuoteRe     = /"/g;

/**
 * Returns a safe property accessor for the specified property name.
 * @param {string} prop Property name
 * @returns {string} Safe accessor
 */
exports.safeProp = function safeProp(prop) {
    if (useNative && native.safeProp) {
        return native.safeProp(prop);
    }
    if (!/^[$\w_]+$/.test(prop) || exports.isReserved(prop))
        return "[\"" + prop.replace(safePropBackslashRe, "\\\\").replace(safePropQuoteRe, "\\\"") + "\"]";
    return "." + prop;
};

/**
 * Converts an object's values to an array.
 * @param {Object.<string,*>} object Object to convert
 * @returns {Array.<*>} Converted array
 */
exports.toArray = function toArray(object) {
    if (useNative && native.toArray) {
        try {
            return native.toArray(object);
        } catch (e) {
            // Fall through to JavaScript implementation
        }
    }
    if (object) {
        var keys  = Object.keys(object),
            array = new Array(keys.length),
            index = 0;
        while (index < keys.length)
            array[index] = object[keys[index++]];
        return array;
    }
    return [];
};

/**
 * Converts an array of keys immediately followed by their respective value to an object, omitting undefined values.
 * @param {Array.<*>} array Array to convert
 * @returns {Object.<string,*>} Converted object
 */
exports.toObject = function toObject(array) {
    if (useNative && native.toObject) {
        try {
            return native.toObject(array);
        } catch (e) {
            // Fall through to JavaScript implementation
        }
    }
    var object = {},
        index  = 0;
    while (index < array.length) {
        var key = array[index++],
            val = array[index++];
        if (val !== undefined)
            object[key] = val;
    }
    return object;
};

/**
 * Check if native module is being used
 * @returns {boolean} true if native module is active
 */
exports.isUsingNative = function isUsingNative() {
    return useNative;
};

/**
 * Get native module info
 * @returns {string} Native module version or 'not available'
 */
exports.getNativeInfo = function getNativeInfo() {
    if (useNative && native.getNativeVersion) {
        return "Native module v" + native.getNativeVersion();
    }
    return "JavaScript fallback";
};
