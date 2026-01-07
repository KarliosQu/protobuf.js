# protobuf.js Rust 移植与优化过程推理文档

## 1. 概述
本文档记录了将 `protobuf.js` 核心逻辑移植到 Rust 并实现高性能优化的完整过程。该工程旨在通过 Rust 的高性能特性解决原版 JavaScript 在处理大量数据编解码时的性能瓶颈。

## 2. 工程初始化与环境搭建
**日期**: 2026-01-07
**操作**: 
1. 建立了 `rust-show-20260106` 开发分支。
2. 在 `protobuf.js` 根目录下创建了 `rust` 目录，用于存放 Rust 源码及构建配置。
3. 确定了使用 N-API (via `napi-rs`) 作为 Rust 与 Node.js 通信的桥梁，这是目前最稳定且性能优异的 Node.js 扩展开发方案。

**决策推理**:
- **为何选择 N-API**: 相比 V8 API，N-API 提供了稳定的 ABI，保证了跨 Node.js 版本的兼容性。`napi-rs` 库提供了极佳的 Rust 封装，降低了开发复杂度。
- **目录结构**: 将 Rust 相关代码隔离在 `rust/` 目录中，保持对原生 `protobuf.js` 代码的最小侵入，便于后续维护和双模式切换。

## 3. 架构设计与分析 (基于 Mirror 版本推理)
通过分析 reference (mirror) 版本，我们确立了以下核心移植目标：
- **双模式切换**: 保留原有纯 JS 实现，允许用户通过环境变量或配置一键开启 Rust 加速模式。
- **核心路径优化**: 重点优化 `Encoder` (序列化) 和 `Decoder` (反序列化) 的热点路径。
- **JS 适配层**: 编写轻量级的 JS Wrapper，拦截原有的 `protobuf.Code` 调用，将其转发给 Rust 扩展。

## 4. 实施日志
*(后续步骤将在开发过程中实时更新)*

### 4.1 基础设施搭建 (2026-01-07)
**操作**: 
- 初始化 `rust` 子项目，配置 `Cargo.toml` 引入 `napi`, `bytes` 等关键依赖。
- 配置 Rust 项目的 `package.json`，确保构建产物能被 Node.js 正确加载。
- 在根目录 `package.json` 添加 `build:rust` 脚本，打通构建流程。

**决策推理**:
- **依赖选择**: `bytes` 库是 Rust 生态中处理二进制流的标准库，适合 Protobuf 的 Buffer 操作。`rayon` 用于潜在的并行计算优化。

### 4.2 核心编解码逻辑移植 (2026-01-07)
**操作**:
- 移植 `Writer` (writer.rs): 实现零拷贝的 Buffer 写入逻辑，集成内存池 (`pool.rs`) 以减少内存分配开销。
- 移植 `Reader` (reader.rs): 实现高效的 Varint 解码和 Buffer 读取。
- 引入内存池 `BufferPool`: 解决高频序列化场景下的内存碎片问题。

**移植细节**:
- 移除了原版中部分未使用的复杂 Ref 逻辑，简化为 `Reader/Writer` 核心。
- 重新实现了 `write_varint32_fast` 等内联热点函数，确保在各个平台上的极致性能。
