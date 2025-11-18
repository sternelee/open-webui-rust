# Open WebUI Rust 后端

Open WebUI Rust后端,比原 Python 后端性能更优、可靠性和可扩展性。

<p align="center">
   <a href="https://www.bilibili.com/video/BV1ci1xB9E2i/" target="_blank" rel="noopener noreferrer">
      <img width="600" src="./img/vid-cover-zh.png" alt="Open WebUI Rust后端">
   </a>
</p>

## Docker快速开始：

```
git clone https://github.com/knoxchat/open-webui-rust.git && cd open-webui-rust
docker compose up -d
```
> 确保Docker和Docker Compose就绪

## 概述

Rust 后端是 Python 后端的直接替代品, 有这些好处:

- **更快的响应时间**10-50倍
- **更低的内存使用率**70%
- **原生并发**使用 Tokio 异步运行时
- **类型安全**防止整类运行时错误
- **零拷贝流式传输**用于聊天生成
- **生产就绪**具有全面的错误处理

## **重要‼️** 您的赞助将会加速与完善项目开发进度：

- **请通过以下二维码进行支付宝或Paypal扫码赞助并微信联系：knoxsale**

<p align="center" style="display: flex; align-items: center; justify-content: center; gap: 20px;">
   <img width="246" src="./img/ali.png" alt="Name">
   <img width="229" src="./img/paypal.png" alt="Name">
</p>

## 赞助者列表

| 名称 | 赞助金额 | 贡献文件 | 享有特权 |
|------|----------|---------|---------|
| [![栢田医疗](./img/baitian.png)](https://baitianjituan.com) | 5000 | 300 | [![栢田医疗](./img/btyl.png)](https://baitianjituan.com) |
| 孔祥康 | 500 | 30 | 微信服务 |
| Knox用户匿名赞助 | 300 | 18 | 微信服务 |
| [Bestming](https://www.mingagent.com) | 100 | 6 | 微信服务 |
| HJPING | 100 | 6 | 微信服务 |
| KingZ | 50 | 2 | 电邮服务 |
| JimLi | 66 | 2 | 电邮服务 |
| shanwu | 50 | 2 | 电邮服务 |
| xixi | 50 | 2 | 电邮服务 |

## 目录

- [架构](#架构)
- [先决条件](#先决条件)
- [安装](#安装)
- [配置](#配置)
- [运行服务器](#运行服务器)
- [API 兼容性](#api-兼容性)
- [性能](#性能)
- [开发](#开发)
- [测试](#测试)
- [部署](#部署)

## 架构

### 技术栈

- **框架**: Actix-Web 4.x (最快的 Web 框架之一)
- **运行时**: Tokio (原生 async/await 运行时)
- **数据库**: PostgreSQL with SQLx (编译时检查的查询)
- **缓存**: Redis with deadpool 连接池
- **认证**: JWT with jsonwebtoken + Argon2/Bcrypt
- **序列化**: Serde (零拷贝反序列化)
- **HTTP 客户端**: Reqwest (异步 HTTP/2 客户端)

### 项目结构

```
rust-backend/
├── src/
│   ├── main.rs              # 应用程序入口点
│   ├── config.rs            # 配置管理
│   ├── db.rs                # 数据库连接池
│   ├── error.rs             # 集中式错误处理
│   ├── models/              # 数据模型 (25+ 实体)
│   │   ├── auth.rs          # 用户、会话、API密钥模型
│   │   ├── chat.rs          # 聊天、消息模型
│   │   ├── model.rs         # AI 模型配置
│   │   └── ...              # 频道、文件、知识库等
│   ├── routes/              # HTTP 路由处理器 (25+ 模块)
│   │   ├── auth.rs          # 认证端点
│   │   ├── chats.rs         # 聊天管理
│   │   ├── openai.rs        # OpenAI 兼容 API
│   │   └── ...              # 音频、图片、工具等
│   ├── services/            # 业务逻辑层 (27+ 服务)
│   │   ├── chat.rs          # 聊天处理服务
│   │   ├── auth.rs          # 认证服务
│   │   ├── rag.rs           # RAG (检索) 服务
│   │   └── ...              # 模型、用户、文件服务
│   ├── middleware/          # 请求/响应中间件
│   │   ├── auth.rs          # JWT 认证
│   │   ├── audit.rs         # 请求审计
│   │   └── rate_limit.rs    # 速率限制
│   ├── utils/               # 实用工具函数
│   │   ├── auth.rs          # JWT 助手
│   │   ├── embeddings.rs    # 向量嵌入
│   │   └── chat_completion.rs # 聊天工具
│   ├── socket.rs            # WebSocket/Socket.IO 支持
│   └── websocket_chat.rs    # 实时聊天流式传输
├── migrations/              # 数据库迁移
│   └── postgres/            # PostgreSQL 架构迁移
├── Cargo.toml               # Rust 依赖项
└── .env.example             # 环境配置模板
```

## 先决条件

- **Rust**: 1.75+ (通过 [rustup](https://rustup.rs/) 安装)
- **PostgreSQL**: 13+ (必需)
- **Redis**: 6.0+ (可选,推荐用于会话和缓存)
- **操作系统**: Linux、macOS 或 Windows

## 安装

### 1. 克隆仓库

```bash
cd rust-backend
```

### 2. 安装 Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### 3. 设置数据库

```bash
# 创建 PostgreSQL 数据库
createdb openwebui

# 设置数据库 URL
export DATABASE_URL="postgresql://postgres:password@localhost:5432/openwebui"
```

### 4. 安装依赖

```bash
# 依赖项由 Cargo 自动管理
cargo fetch
```

## 配置

### 使用 Docker Compose 进行开发

使用所有服务（PostgreSQL、Redis、ChromaDB）进行本地开发：

```bash
# 启动所有开发服务
docker compose -f docker-compose.dev.yaml up -d

# 查看日志
docker compose -f docker-compose.dev.yaml logs -f

# 停止服务
docker compose -f docker-compose.dev.yaml down

# 停止服务并删除卷（全新开始）
docker compose -f docker-compose.dev.yaml down -v
```

**包含的服务：**
- **PostgreSQL**（端口 5432）：主数据库
- **Redis**（端口 6379）：缓存和会话管理
- **ChromaDB**（端口 8000）：用于 RAG/嵌入的向量数据库
- **pgAdmin**（端口 5050）：PostgreSQL 管理界面（可选，使用 `--profile tools`）
- **Redis Commander**（端口 8082）：Redis 管理界面（可选，使用 `--profile tools`）

### Rust 后端环境变量

在 `rust-backend/` 目录中创建 `.env` 文件：

查看 `rust-backend/env.example` 获取完整的配置选项。

## 运行服务器

### 使用 Docker 服务的开发模式

**步骤 1：启动开发服务**

```bash
# 启动 PostgreSQL、Redis 和 ChromaDB
docker compose -f docker-compose.dev.yaml up -d

# 验证服务正在运行
docker compose -f docker-compose.dev.yaml ps
```

**步骤 3：运行 Rust 后端**

```bash
cargo run
```

服务器将在 `http://0.0.0.0:8080` 启动

**此设置的优势：**
- ✅ 所有依赖项（PostgreSQL、Redis、ChromaDB）在 Docker 中运行
- ✅ Rust 后端本地运行，编译和调试更快
- ✅ JWT 令牌在后端重启时保持有效（使用固定的 WEBUI_SECRET_KEY）
- ✅ 使用 `docker compose down -v` 轻松重置数据库

## 性能

### 快速摘要

| 指标 | Python (FastAPI) | Rust (Actix-Web) | 改进 |
|--------|------------------|------------------|-------------|
| 登录 (p50) | 45ms | 3ms | **快 15 倍** |
| 聊天生成 (p50) | 890ms | 35ms* | **快 25 倍** |
| 模型列表 (p50) | 23ms | 1.2ms | **快 19 倍** |
| 内存 (1000 请求) | 450 MB | 85 MB | **降低 5.3 倍** |
| 吞吐量 | 850 请求/秒 | 12,400 请求/秒 | **提高 14.6 倍** |

*注意: 聊天生成速度主要取决于 LLM 提供商。Rust 在流式传输和处理开销方面表现出色。

## 开发

### 先决条件

```bash
# 安装开发工具
rustup component add rustfmt clippy

# 安装 cargo-watch 用于自动重新加载
cargo install cargo-watch
```

### 开发工作流

```bash
# 确保 Docker 服务正在运行
docker compose -f docker-compose.dev.yaml up -d

# 文件更改时自动重新加载
cd rust-backend
cargo watch -x run

# 运行测试
cargo test

# 运行带输出的测试
cargo test -- --nocapture

# 格式化代码
cargo fmt

# 检查代码
cargo clippy -- -D warnings

# 不构建的情况下检查
cargo check
```

## 测试

### 单元测试

```bash
cargo test --lib
```

### 集成测试

```bash
# 设置测试数据库
export DATABASE_URL=postgresql://postgres:postgres@localhost:5432/openwebui_test

# 运行集成测试
cargo test --test '*'
```
### 负载测试

```bash
# 安装 wrk
brew install wrk  # macOS
sudo apt install wrk  # Ubuntu

# 运行负载测试
wrk -t4 -c100 -d30s --latency http://localhost:8080/health
```

## 部署

### 生产构建

```bash
# 构建优化的二进制文件
cargo build --release

# 二进制文件位置
./target/release/open-webui-rust

# 去除符号 (减小大小)
strip ./target/release/open-webui-rust
```