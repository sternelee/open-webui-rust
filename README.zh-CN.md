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

**docker-compose.dev.yaml 的环境变量：**

在项目根目录创建 `.env` 文件以自定义：

```bash
# PostgreSQL
POSTGRES_DB=openwebui
POSTGRES_USER=postgres
POSTGRES_PASSWORD=postgres
POSTGRES_PORT=5432

# Redis
REDIS_PORT=6379

# ChromaDB
CHROMA_PORT=8000
CHROMA_TELEMETRY=FALSE

# 管理工具（可选）
PGADMIN_EMAIL=admin@admin.com
PGADMIN_PASSWORD=admin
PGADMIN_PORT=5050
REDIS_COMMANDER_USER=admin
REDIS_COMMANDER_PASSWORD=admin
REDIS_COMMANDER_PORT=8082
```

**启动管理工具：**

```bash
docker compose -f docker-compose.dev.yaml --profile tools up -d
```

### Rust 后端环境变量

在 `rust-backend/` 目录中创建 `.env` 文件：

```bash
# 服务器配置
HOST=0.0.0.0
PORT=8080
ENV=development
RUST_LOG=info

# 安全（重要：设置固定密钥以在重启时保持认证令牌）
WEBUI_SECRET_KEY=your-secret-key-min-32-chars
JWT_EXPIRES_IN=168h

# 数据库（必需）- 匹配 docker-compose.dev.yaml 设置
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/openwebui
DATABASE_POOL_SIZE=10
DATABASE_POOL_MAX_OVERFLOW=10
DATABASE_POOL_TIMEOUT=30
DATABASE_POOL_RECYCLE=3600

# Redis（推荐）- 匹配 docker-compose.dev.yaml 设置
ENABLE_REDIS=false
REDIS_URL=redis://localhost:6379

# 认证
ENABLE_SIGNUP=true
ENABLE_LOGIN_FORM=true
ENABLE_API_KEY=true
DEFAULT_USER_ROLE=pending

# OpenAI 配置（如果使用 OpenAI 模型）
ENABLE_OPENAI_API=true
OPENAI_API_KEY=sk-your-key
OPENAI_API_BASE_URL=https://api.openai.com/v1

# CORS
CORS_ALLOW_ORIGIN=*

# WebSocket
ENABLE_WEBSOCKET_SUPPORT=true
WEBSOCKET_MANAGER=local

# 功能
ENABLE_IMAGE_GENERATION=false
ENABLE_CODE_EXECUTION=false
ENABLE_WEB_SEARCH=false

# 音频（可选）
TTS_ENGINE=openai
STT_ENGINE=openai

# RAG/检索（可选）- ChromaDB 集成
CHUNK_SIZE=1500
CHUNK_OVERLAP=100
RAG_TOP_K=5

# 存储
UPLOAD_DIR=/app/data/uploads

# 日志
GLOBAL_LOG_LEVEL=INFO
```

**重要提示：**
- **WEBUI_SECRET_KEY**：必须设置为固定值（至少 32 个字符）以防止服务器重启时 JWT 令牌失效。使用 `uuidgen` 或生成安全的随机字符串。
- **DATABASE_URL**：应与 `docker-compose.dev.yaml` 中的 PostgreSQL 凭据匹配
- **REDIS_URL**：应与 `docker-compose.dev.yaml` 中的 Redis 端口匹配

查看 `rust-backend/env.example` 获取完整的配置选项。

### 配置优先级

1. 环境变量 (最高优先级)
2. `.env` 文件
3. 数据库存储的配置
4. 默认值 (最低优先级)

## 运行服务器

### 使用 Docker 服务的开发模式

**步骤 1：启动开发服务**

```bash
# 启动 PostgreSQL、Redis 和 ChromaDB
docker compose -f docker-compose.dev.yaml up -d

# 验证服务正在运行
docker compose -f docker-compose.dev.yaml ps
```

**步骤 2：配置 Rust 后端**

```bash
cd rust-backend

# 复制示例环境文件
cp env.example .env

# 编辑 .env 并将 WEBUI_SECRET_KEY 设置为固定值
# 示例：WEBUI_SECRET_KEY=$(uuidgen | tr '[:upper:]' '[:lower:]')
nano .env
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

### 生产模式 (优化)

```bash
cargo run --release
```

### 使用构建脚本

```bash
./build.sh          # 构建发布版二进制文件
./build.sh --dev    # 构建调试版二进制文件
./build.sh --run    # 构建并运行
```

### Docker

```bash
docker build -t open-webui-rust .
docker run -p 8080:8080 --env-file .env open-webui-rust
```

## API 兼容性

Rust 后端对核心端点保持与 Python 后端 **100% API 兼容性**:

### OAuth 2.0 / OpenID Connect
- `GET /oauth/{provider}/login` - 发起 OAuth 登录 (Google, Microsoft, GitHub, OIDC, Feishu)
- `GET /oauth/{provider}/callback` - OAuth 回调处理
- `GET /oauth/{provider}/login/callback` - 登录回调端点
- `GET /api/v1/users/{id}/oauth/sessions` - 获取用户 OAuth 会话

### 认证
- `POST /api/v1/auths/signup` - 用户注册
- `POST /api/v1/auths/signin` - 用户登录
- `POST /api/v1/auths/signout` - 用户登出
- `POST /api/v1/auths/api_key` - 生成 API 密钥

### 聊天生成
- `POST /api/chat/completions` - OpenAI 兼容的聊天
- `POST /api/v1/chat/completions` - 替代端点
- `POST /openai/v1/chat/completions` - 完全 OpenAI 兼容
- `WS /api/ws/chat` - WebSocket 流式传输

### 模型
- `GET /api/models` - 列出可用模型
- `GET /api/models/base` - 列出基础模型
- `POST /api/v1/models` - 创建模型
- `GET /api/v1/models/:id` - 获取模型详情

### 用户
- `GET /api/v1/users` - 列出用户 (管理员)
- `GET /api/v1/users/:id` - 获取用户资料
- `PUT /api/v1/users/:id` - 更新用户
- `DELETE /api/v1/users/:id` - 删除用户

### 文件与知识库
- `POST /api/v1/files` - 上传文件
- `GET /api/v1/files/:id` - 下载文件
- `POST /api/v1/knowledge` - 创建知识库
- `GET /api/v1/retrieval/query` - 查询知识

### 健康与状态
- `GET /health` - 基本健康检查
- `GET /health/db` - 数据库连接检查
- `GET /api/config` - 前端配置
- `GET /api/version` - 后端版本

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

# 查看 Docker 服务日志
docker compose -f docker-compose.dev.yaml logs -f postgres
docker compose -f docker-compose.dev.yaml logs -f redis
docker compose -f docker-compose.dev.yaml logs -f chromadb
```

### 代码结构指南

1. **模型** (`src/models/`): 带有 Serde 序列化的数据库实体
2. **服务** (`src/services/`): 业务逻辑,可跨路由重用
3. **路由** (`src/routes/`): HTTP 处理器,调用服务的薄层
4. **中间件** (`src/middleware/`): 横切关注点 (认证、日志记录)
5. **工具** (`src/utils/`): 助手函数,无业务逻辑

### 添加新功能

1. 在 `src/models/[feature].rs` 中添加模型
2. 在 `migrations/postgres/` 中添加数据库迁移
3. 在 `src/services/[feature].rs` 中实现服务
4. 在 `src/routes/[feature].rs` 中添加路由
5. 在 `src/routes/mod.rs` 中注册路由
6. 添加测试

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

### 使用演示账户测试

```bash
# 后端包含一个演示账户
# 邮箱: test@test.com
# 密码: test1234
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