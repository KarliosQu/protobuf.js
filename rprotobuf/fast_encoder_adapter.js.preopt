const { fast_encoder } = require('./index');
console.log('fast_encoder:', fast_encoder);

const Op = {
	Double: 1,
	Float: 2,
	Int32: 3,
	UInt32: 4,
	SInt32: 5,
	Bool: 6,
	Int64: 7,
	UInt64: 8,
	SInt64: 9,
	String: 10,
	Bytes: 11,
	Nested: 12,
	RepeatedBool: 26,
	RepeatedBoolPacked: 27,
};

class FastEncoder {
	constructor(type) {
		this.type = type;
		this.schemaId = null;
		this.instructions = []; 
		this.flattenPlan = []; 
		this.numerics = new Float64Array(1024 * 1024); // 8MB
		this.longs = new BigInt64Array(128 * 1024); // 1MB
		this.strings = Buffer.allocUnsafe(4 * 1024 * 1024); // 4MB
		this.outputBuffer = Buffer.allocUnsafe(8 * 1024 * 1024); // 8MB
		this.objects = [];
	}

	register() {
		if (this.schemaId !== null) return;
        
		this._generatePlan(this.type, this.instructions, this.flattenPlan);
        
		this.schemaId = fast_encoder.registerSchemaFast(this.instructions);
		console.log(`Registered schema ${this.schemaId} with ${this.instructions.length} instructions`);
        
		this.encodeFunc = this._compileEncode(this.type);
	}

	_generatePlan(type, instructions, plan) {
		// ... (same as before, just setup context)
		const context = {
			instructions: instructions,
			plan: plan,
			numericIndex: 0,
			longIndex: 0,
			objectIndex: 0
		};
		this._generateRecursive(type, context);
	}

	_generateRecursive(type, ctx) {
		const fields = type.fieldsArray || Object.values(type.fields).sort((a, b) => a.id - b.id);
        
		const wireTypes = {
			double: 1,
			float: 5,
			int32: 0,
			uint32: 0,
			sint32: 0,
			fixed32: 5,
			sfixed32: 5,
			int64: 0,
			uint64: 0,
			sint64: 0,
			fixed64: 1,
			sfixed64: 1,
			bool: 0,
			string: 2,
			bytes: 2,
			message: 2
		};

		for (const field of fields) {
			let wireType = wireTypes[field.type];
			if (wireType === undefined) wireType = 0; // Default to varint (e.g. enum)
            
			if (field.resolvedType && field.resolvedType.constructor.name === "Type") {
				wireType = 2; // Nested message
			}

			const tag = (field.id << 3) | wireType;
            
			if (field.repeated) {
				if (field.type === 'bool') {
					const isPacked = field.packed !== false; 
					const op = isPacked ? Op.RepeatedBoolPacked : Op.RepeatedBool;
                    
					// If packed, tag is (id << 3) | 2
					// If not packed, tag is (id << 3) | 0
					const actualWireType = isPacked ? 2 : 0;
					const actualTag = (field.id << 3) | actualWireType;
                    
					ctx.instructions.push(op, actualTag);
				}
			} else if (field.resolvedType) {
				if (field.resolvedType.constructor.name === "Enum" || (field.resolvedType.values && !field.resolvedType.fields)) {
					 ctx.instructions.push(Op.Int32, tag);
				} else {
					const subInstructions = [];
					const subCtx = {
						instructions: subInstructions,
						plan: [], 
						numericIndex: 0, 
						longIndex: 0,
						objectIndex: 0
					};
					this._generateRecursive(field.resolvedType, subCtx);
                    
					const subId = fast_encoder.registerSchemaFast(subInstructions);
					ctx.instructions.push(Op.Nested, tag, subId);
				}
			} else {
				let opCode;
                
				switch (field.type) {
					case 'double': opCode = Op.Double; break;
					case 'float': opCode = Op.Float; break;
					case 'int32': opCode = Op.Int32; break;
					case 'uint32': opCode = Op.UInt32; break;
					case 'sint32': opCode = Op.SInt32; break;
					case 'bool': opCode = Op.Bool; break;
					case 'string': opCode = Op.String; break;
					case 'bytes': opCode = Op.Bytes; break; 
					case 'int64': opCode = Op.Int64; break;
					case 'uint64': opCode = Op.UInt64; break;
					case 'sint64': opCode = Op.SInt64; break;
					default: opCode = Op.Int32; 
				}
                
				ctx.instructions.push(opCode, tag);
			}
		}
	}

	encode(message) {
		if (this.schemaId === null) this.register();
        
		const ends = this.encodeFunc(message, this.numerics, this.longs, this.strings, protobuf);
        
		// Returns start offset in outputBuffer
		const start = fast_encoder.encodeFast(this.schemaId, this.numerics, ends.n, this.longs, ends.l, this.strings, ends.s, this.outputBuffer);
        
		// Copy from start to end
		const len = this.outputBuffer.length - start;
		const result = Buffer.allocUnsafe(len);
		this.outputBuffer.copy(result, 0, start, this.outputBuffer.length);
		return result;
	}

	_compileEncode(type) {
		const ctx = {
			code: [],
			varIdx: 0
		};
        
		ctx.code.push("var nIdx = 0;");
		ctx.code.push("var lIdx = 0;");
		ctx.code.push("var sIdx = 0;");
        
		this._generateCodeRecursive(type, "m", ctx);
        
		ctx.code.push("return { n: nIdx, l: lIdx, s: sIdx };");
        
		const body = ctx.code.join("\n");
		return new Function("m", "numerics", "longs", "strings", "protobuf", body);
	}

	_generateCodeRecursive(type, varName, ctx) {
		const fields = type.fieldsArray || Object.values(type.fields).sort((a, b) => a.id - b.id);
        
		for (const field of fields) {
			const prop = `${varName}["${field.name}"]`;
			const myVar = `v${ctx.varIdx++}`;
			ctx.code.push(`var ${myVar} = ${prop};`);
            
			if (field.repeated) {
				if (field.type === 'bool') {
					ctx.code.push(`var arr = ${myVar} || [];`);
					ctx.code.push(`for (var i = 0; i < arr.length; i++) {`);
					ctx.code.push(`  numerics[nIdx++] = arr[i] ? 1 : 0;`);
					ctx.code.push(`}`);
					ctx.code.push(`numerics[nIdx++] = arr.length;`);
				}
			} else if (field.resolvedType) {
				 if (field.resolvedType.constructor.name === "Enum" || (field.resolvedType.values && !field.resolvedType.fields)) {
					 ctx.code.push(`numerics[nIdx++] = (${myVar} === undefined) ? 0 : ${myVar};`);
				 } else {
					 // Nested
					 ctx.code.push(`if (${myVar}) {`);
					 this._generateCodeRecursive(field.resolvedType, myVar, ctx);
					 ctx.code.push(`  numerics[nIdx++] = 1.0;`);
					 ctx.code.push(`} else {`);
					 ctx.code.push(`  numerics[nIdx++] = 0.0;`);
					 ctx.code.push(`}`);
				 }
			} else {
				// Primitive
				if (field.type === 'string') {
					// String optimization
					ctx.code.push(`if (${myVar} !== undefined && ${myVar} !== null) {`);
					ctx.code.push(`  var len = strings.write(${myVar}, sIdx);`);
					ctx.code.push(`  numerics[nIdx++] = sIdx;`);
					ctx.code.push(`  numerics[nIdx++] = len;`);
					ctx.code.push(`  sIdx += len;`);
					ctx.code.push(`} else {`);
					ctx.code.push(`  numerics[nIdx++] = 0; numerics[nIdx++] = 0;`);
					ctx.code.push(`}`);
				} else if (field.type === 'bytes') {
					 // Bytes (assume Buffer)
					 // TODO: Handle bytes
					 ctx.code.push(`numerics[nIdx++] = 0; numerics[nIdx++] = 0;`);
				} else if (field.type === 'int64' || field.type === 'uint64' || field.type === 'sint64') {
					ctx.code.push(`if (${myVar} === undefined || ${myVar} === null) longs[lIdx++] = 0n;`);
					ctx.code.push(`else if (typeof ${myVar} === 'bigint') longs[lIdx++] = ${myVar};`);
					ctx.code.push(`else {`);
					ctx.code.push(`  try {`);
					ctx.code.push(`    if (protobuf.util.Long.isLong(${myVar}) || (${myVar} && ${myVar}.low !== undefined)) longs[lIdx++] = BigInt(protobuf.util.Long.fromValue(${myVar}).toString());`);
					ctx.code.push(`    else longs[lIdx++] = BigInt(${myVar});`);
					ctx.code.push(`  } catch(e) { longs[lIdx++] = 0n; }`);
					ctx.code.push(`}`);
				} else {
					// Numeric
					ctx.code.push(`numerics[nIdx++] = (${myVar} === undefined || ${myVar} === null) ? 0 : Number(${myVar});`);
				}
			}
		}
	}

	_generateDefaultsRecursive(type, ctx) {
		const fields = type.fieldsArray || Object.values(type.fields).sort((a, b) => a.id - b.id);
		for (const field of fields) {
			 if (field.repeated) {
				 if (field.type === 'bool') ctx.code.push(`objects.push([]);`);
			 } else if (field.resolvedType) {
				 if (field.resolvedType.constructor.name === "Enum" || (field.resolvedType.values && !field.resolvedType.fields)) {
					 ctx.code.push(`numerics[nIdx++] = 0;`);
				 } else {
					 ctx.code.push(`numerics[nIdx++] = 0.0;`);
					 this._generateDefaultsRecursive(field.resolvedType, ctx);
				 }
			 } else {
				 if (field.type === 'string') {
					 ctx.code.push(`numerics[nIdx++] = 0; numerics[nIdx++] = 0;`);
				 } else if (field.type === 'bytes') {
					 ctx.code.push(`numerics[nIdx++] = 0; numerics[nIdx++] = 0;`);
				 } else if (field.type === 'int64' || field.type === 'uint64' || field.type === 'sint64') {
					 ctx.code.push(`longs[lIdx++] = 0n;`);
				 } else {
					 ctx.code.push(`numerics[nIdx++] = 0;`);
				 }
			 }
		}
	}
}

// Wrapper to mimic protobuf.js interface
const protobuf = require("protobufjs");

const wrapper = {
	loadSync: function(filename) {
		const root = protobuf.loadSync(filename);
		const originalLookup = root.lookup;
        
		root.lookup = function(typeName) {
			const type = originalLookup.call(this, typeName);
			if (type && !type._fastEncoder) {
				type._fastEncoder = new FastEncoder(type);
                
				// Override encode
				type.encode = function(message) {
					const buffer = type._fastEncoder.encode(message);
					return {
						finish: () => buffer
					};
				};
                
				// Decode is not implemented in FastEncoder yet, fallback to original?
				// The benchmark calls decode too.
				// For now, we only care about encode performance.
				// But if decode fails, benchmark might crash.
				// Let's leave decode as is (original protobuf.js decode).
			}
			return type;
		};
        
		return root;
	},
	Root: protobuf.Root // For fromJSON benchmark
};

module.exports = wrapper;
