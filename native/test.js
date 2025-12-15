// Simple test for the native module
const native = require('./index.js');

console.log('Testing native module...');
console.log('');

// Test init function
try {
    const initResult = native.init();
    console.log('✓ init():', initResult);
} catch (error) {
    console.error('✗ init() failed:', error.message);
}

// Test version function
try {
    const version = native.getNativeVersion();
    console.log('✓ getNativeVersion():', version);
} catch (error) {
    console.error('✗ getNativeVersion() failed:', error.message);
}

// Test camelCase function
try {
    const result = native.camelCase('hello_world');
    console.log('✓ camelCase("hello_world"):', result);
    if (result !== 'helloWorld') {
        console.warn('  Warning: Expected "helloWorld", got', result);
    }
} catch (error) {
    console.error('✗ camelCase() failed:', error.message);
}

// Test ucFirst function
try {
    const result = native.ucFirst('hello');
    console.log('✓ ucFirst("hello"):', result);
    if (result !== 'Hello') {
        console.warn('  Warning: Expected "Hello", got', result);
    }
} catch (error) {
    console.error('✗ ucFirst() failed:', error.message);
}

// Test isReserved function
try {
    const result1 = native.isReserved('if');
    const result2 = native.isReserved('myVar');
    console.log('✓ isReserved("if"):', result1, '(should be true)');
    console.log('✓ isReserved("myVar"):', result2, '(should be false)');
} catch (error) {
    console.error('✗ isReserved() failed:', error.message);
}

// Test safeProp function
try {
    const result1 = native.safeProp('myProp');
    const result2 = native.safeProp('class');
    console.log('✓ safeProp("myProp"):', result1);
    console.log('✓ safeProp("class"):', result2);
} catch (error) {
    console.error('✗ safeProp() failed:', error.message);
}

console.log('');
console.log('Native module tests completed!');
