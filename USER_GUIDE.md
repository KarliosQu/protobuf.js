# Protobuf-Rust 用户指南

本版本的 `protobuf.js` 集成了 Rust 编写的高性能底层核心。在保持 API 完全兼容的前提下，为高负载场景提供显著的性能提升。

## 1. 快速开始

### 1.1 前置要求
- Node.js >= 16
- Rust 开发环境 (推荐安装 rustup，仅在源码编译时需要)

### 1.2 安装与构建
如果你是从源码安装，请在项目根目录下运行：

```bash
# 安装依赖
npm install

# 编译 Rust 扩展
npm run build:rust
```

### 1.3 启用 Rust 加速模式
默认情况下，库依然运行在纯 JavaScript 模式下，以保证最大兼容性。
要启用 Rust 加速，目前支持通过环境变量控制：

**Windows (PowerShell):**
```powershell
$env:PROTOBUF_USE_RUST="true"; node your_script.js
```

**Linux / macOS:**
```bash
PROTOBUF_USE_RUST=true node your_script.js
```

或者在代码最开始显式配置：

```javascript
const protobuf = require("protobufjs");
// 尝试加载 Rust 扩展（如果可用）
protobuf.util.useRust = true; 
```

## 2. 性能对比
在基准测试中，Rust 模式在大量小消息序列化场景下通常能获得 2-5 倍的性能提升，在反序列化场景下得益于可以直接操作内存，性能提升更为明显。不仅 CPU 占用降低，在大对象处理上的 GC 压力也显著减小。

## 3. 常见问题
**Q: 启用 Rust 模式后报错 "Cannot find module ... .node"**
A: 请确保你已经运行了 `npm run build:rust` 成功编译了 Rust 扩展。

**Q: 是否支持所有 protobuf 特性？**
A: 目前核心的编解码功能已支持，部分高级反射特性或极其动态的用法可能仍会回退到 JS 实现。
