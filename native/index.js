// Native module loader
// This file loads the Rust native module and exports its functions

try {
    const native = require('./index.node');
    module.exports = native;
} catch (error) {
    // Fallback if native module is not available
    console.warn('Native Rust module not available, falling back to JavaScript implementation:', error.message);
    module.exports = {
        init: () => 'protobufjs JavaScript fallback',
        getNativeVersion: () => '0.0.0',
        camelCase: (str) => {
            return str.replace(/_([a-z])/g, (match, letter) => letter.toUpperCase());
        },
        ucFirst: (str) => {
            return str.charAt(0).toUpperCase() + str.slice(1);
        },
        isReserved: (name) => {
            const reserved = [
                'do', 'if', 'in', 'for', 'let', 'new', 'try', 'var', 'case', 'else', 'enum',
                'eval', 'false', 'null', 'this', 'true', 'void', 'with', 'break', 'catch',
                'class', 'const', 'super', 'throw', 'while', 'yield', 'delete', 'export',
                'import', 'public', 'return', 'static', 'switch', 'typeof', 'default', 'extends',
                'finally', 'package', 'private', 'continue', 'debugger', 'function', 'arguments',
                'interface', 'protected', 'implements', 'instanceof'
            ];
            return reserved.includes(name);
        },
        safeProp: (prop) => {
            const isSafe = /^[$\w_]+$/.test(prop) && !module.exports.isReserved(prop);
            if (isSafe && prop) {
                return '.' + prop;
            }
            return '["' + prop.replace(/\\/g, '\\\\').replace(/"/g, '\\"') + '"]';
        },
        toArray: (obj) => {
            return Object.values(obj);
        },
        toObject: (array) => {
            const result = {};
            for (let i = 0; i + 1 < array.length; i += 2) {
                const key = array[i];
                const val = array[i + 1];
                if (val !== undefined && val !== null) {
                    result[key] = val;
                }
            }
            return result;
        }
    };
}
