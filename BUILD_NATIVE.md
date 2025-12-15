# Building the Native Rust Module

This guide provides detailed instructions for building the protobuf.js native Rust module.

## Prerequisites

### Required Tools

1. **Rust Toolchain** (1.56.0 or later)
   ```bash
   # Install Rust via rustup
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   
   # Verify installation
   cargo --version
   rustc --version
   ```

2. **Node.js** (12.0.0 or later)
   ```bash
   node --version
   npm --version
   ```

3. **Build Tools**
   
   **Linux:**
   ```bash
   # Debian/Ubuntu
   sudo apt-get install build-essential
   
   # Fedora/RHEL
   sudo yum groupinstall "Development Tools"
   ```
   
   **macOS:**
   ```bash
   # Install Xcode Command Line Tools
   xcode-select --install
   ```
   
   **Windows:**
   - Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/)
   - Ensure "Desktop development with C++" workload is selected

## Quick Build

```bash
# Install dependencies
npm install

# Build the native module
npm run build:native

# Build everything (native + bundles + types)
npm run build
```

## Build Commands

### Development Build

```bash
# Debug build (faster compilation, slower runtime)
npm run build:native:debug

# Or directly with cargo
cd native
cargo build
```

### Production Build

```bash
# Release build (slower compilation, faster runtime)
npm run build:native

# Or directly with cargo
cd native
cargo build --release
```

### Testing

```bash
# Run Rust tests
cd native
cargo test

# Run native module integration test
node native/test.js

# Run full protobuf.js test suite
npm test
```

## Cross-Compilation

### Linux

```bash
# x86_64 (GNU libc)
rustup target add x86_64-unknown-linux-gnu
cargo build --release --target x86_64-unknown-linux-gnu

# x86_64 (musl - static linking)
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl

# ARM64/aarch64 (GNU libc)
rustup target add aarch64-unknown-linux-gnu
cargo build --release --target aarch64-unknown-linux-gnu

# ARM64/aarch64 (musl)
rustup target add aarch64-unknown-linux-musl
cargo build --release --target aarch64-unknown-linux-musl
```

### macOS

```bash
# Intel x86_64
rustup target add x86_64-apple-darwin
cargo build --release --target x86_64-apple-darwin

# Apple Silicon (M1/M2/M3)
rustup target add aarch64-apple-darwin
cargo build --release --target aarch64-apple-darwin

# Universal binary (both architectures)
# Build both targets and use lipo to combine
cargo build --release --target x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin
lipo -create \
  target/x86_64-apple-darwin/release/libprotobufjs_native.dylib \
  target/aarch64-apple-darwin/release/libprotobufjs_native.dylib \
  -output native/index.node
```

### Windows

```bash
# x86_64 (MSVC)
rustup target add x86_64-pc-windows-msvc
cargo build --release --target x86_64-pc-windows-msvc

# x86_64 (GNU)
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu
```

## Output Files

After building, the native module will be located at:

```
native/
├── target/
│   ├── debug/                      # Debug builds
│   │   └── libprotobufjs_native.{so,dylib,dll}
│   └── release/                    # Release builds
│       └── libprotobufjs_native.{so,dylib,dll}
└── index.node                      # Symlink/copy for Node.js loading
```

File extensions by platform:
- **Linux**: `.so` (shared object)
- **macOS**: `.dylib` (dynamic library)
- **Windows**: `.dll` (dynamic link library)

## Installation Locations

For distribution, the compiled native module should be copied to:

```
native/index.node
```

The JavaScript loader (`native/index.js`) automatically loads this file.

## Build Optimization

### Release Profile Settings

The `Cargo.toml` includes optimized release settings:

```toml
[profile.release]
opt-level = 3        # Maximum optimization
lto = true          # Link-time optimization
codegen-units = 1   # Single codegen unit for best optimization
strip = true        # Strip symbols for smaller binary
```

### Size Optimization

For smaller binary size:

```bash
# Build with size optimization
RUSTFLAGS="-C opt-level=z" cargo build --release

# Further strip the binary (Linux/macOS)
strip target/release/libprotobufjs_native.so
```

### Performance Optimization

For maximum performance:

```bash
# Build with native CPU features
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

## Continuous Integration

### GitHub Actions Example

```yaml
name: Build Native Module

on: [push, pull_request]

jobs:
  build:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          
      - name: Setup Node.js
        uses: actions/setup-node@v3
        with:
          node-version: '18'
          
      - name: Install dependencies
        run: npm install
        
      - name: Build native module
        run: npm run build:native
        
      - name: Run tests
        run: npm test
        
      - name: Upload artifacts
        uses: actions/upload-artifact@v3
        with:
          name: native-${{ matrix.os }}
          path: native/index.node
```

## Troubleshooting

### "cannot find -lnode" error

**Solution**: Ensure Node.js development headers are available:

```bash
# The headers should be in node_modules or system Node.js installation
npm install --build-from-source
```

### "linking with `cc` failed" error

**Solution**: Install C/C++ compiler:

```bash
# Linux
sudo apt-get install build-essential

# macOS
xcode-select --install

# Windows
# Install Visual Studio Build Tools
```

### Permission denied when copying binary

**Solution**: Ensure you have write permissions:

```bash
chmod +w native/index.node
rm native/index.node
npm run build:native
```

### Binary incompatible with system

**Solution**: Rebuild for your specific platform:

```bash
cd native
cargo clean
cargo build --release
cd ..
npm run build:native
```

### Rust not found

**Solution**: Install Rust and add to PATH:

```bash
# Install
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add to PATH (add to ~/.bashrc or ~/.zshrc)
export PATH="$HOME/.cargo/bin:$PATH"

# Reload shell
source ~/.bashrc  # or source ~/.zshrc
```

## Advanced Topics

### Custom Cargo Features

Add features to `Cargo.toml`:

```toml
[features]
default = ["json"]
json = ["serde_json"]
fast-math = []
```

Build with features:

```bash
cargo build --release --features fast-math
```

### Static Linking

For fully static binaries (Linux):

```bash
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
```

### Profiling

```bash
# Build with debug info
cargo build --release --profile release-with-debug

# Profile with perf (Linux)
perf record --call-graph dwarf node examples/native_usage.js
perf report
```

## Distribution

### NPM Package

The native module can be included in the npm package:

1. Add to `package.json` files list:
   ```json
   "files": [
     "native/index.node",
     "native/index.js",
     "native/Cargo.toml",
     "native/src/**"
   ]
   ```

2. Build during `prepublishOnly`:
   ```json
   "scripts": {
     "prepublishOnly": "npm run build"
   }
   ```

### Platform-Specific Packages

Create separate packages for each platform:
- `protobufjs-native-linux`
- `protobufjs-native-darwin`
- `protobufjs-native-win32`

## Support

For build issues:
1. Check [GitHub Issues](https://github.com/protobufjs/protobuf.js/issues)
2. Verify Rust and Node.js versions
3. Try cleaning and rebuilding: `cargo clean && npm run build:native`
4. Check system requirements match your platform

## License

BSD-3-Clause (same as protobuf.js)
