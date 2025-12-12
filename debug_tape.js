const tape = require('tape');

class MyClass {
    constructor(props) {
        Object.assign(this, props);
    }
}

tape('test', t => {
    const plain = { a: 1 };
    const instance = new MyClass({ a: 1 });
    
    t.same(instance, plain, 'instance vs plain');
    t.end();
});
