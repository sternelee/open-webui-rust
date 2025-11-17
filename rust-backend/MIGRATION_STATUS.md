# Axum and Turso Migration - Implementation Plan

## 任务说明 (Task Description)

将 Rust 后端架构从 actix-web 迁移到 axum，并将数据库从 PostgreSQL 迁移到 Turso (libSQL)。

Migrate the Rust backend architecture from actix-web to axum and the database from PostgreSQL to Turso (libSQL).

## 已完成的工作 (Completed Work)

### 1. 依赖更新 (Dependency Updates)
- ✅ 更新了 `Cargo.toml`，添加了 axum、tower、tower-http 等依赖
- ✅ 添加了 libsql 数据库客户端
- ✅ 移除了 actix-web 和 sqlx 的 PostgreSQL 依赖

Updated `Cargo.toml` with axum, tower, tower-http, and libsql dependencies, removing actix-web and PostgreSQL-related dependencies.

### 2. 数据库层重构 (Database Layer Refactoring)
- ✅ 重写了 `src/db.rs`，使用 libSQL 而不是 sqlx
- ✅ 实现了 Turso 本地和远程数据库连接支持
- ✅ 更新了数据库迁移系统以支持 SQLite

Rewrote `src/db.rs` to use libSQL instead of sqlx, with support for both local and remote Turso databases.

### 3. 数据库迁移文件 (Database Migrations)
- ✅ 创建了 `migrations/sqlite/` 目录
- ✅ 转换了第一个迁移文件 (`001_initial.sql`) 从 PostgreSQL 到 SQLite 语法
- ✅ 创建了其他迁移文件的占位符

Converted the initial database migration from PostgreSQL to SQLite syntax and created placeholders for others.

### 4. 示例 Axum 应用 (Example Axum Application)
- ✅ 创建了 `src/main_axum.rs` 作为迁移模板
- ✅ 展示了如何使用 axum 设置路由、中间件和状态管理
- ✅ 包含了基本的健康检查和配置端点

Created `src/main_axum.rs` as a migration template showing route setup, middleware, and state management.

### 5. 迁移文档 (Migration Documentation)
- ✅ 创建了详细的 `MIGRATION_GUIDE.md`
- ✅ 记录了所有需要更新的文件和模式
- ✅ 提供了代码示例展示迁移前后的对比

Created comprehensive `MIGRATION_GUIDE.md` documenting all files to update and migration patterns.

## 需要继续的工作 (Remaining Work)

### 关键文件需要更新 (Critical Files to Update)

1. **服务层** (`src/services/*.rs`) - 约 25+ 文件
   - 所有数据库查询从 sqlx 改为 libSQL API
   - 参数绑定从 `$1, $2` 改为 `?`
   - 手动解析行数据而不是使用 `query_as!` 宏

2. **路由处理器** (`src/routes/*.rs`) - 约 20+ 文件
   - 更新函数签名以使用 axum 提取器
   - 更改响应类型从 `HttpResponse` 到 `Json` 或其他 axum 响应
   - 更新错误处理

3. **中间件** (`src/middleware/*.rs`)
   - 从 actix-web 中间件迁移到 tower 中间件
   - 重写身份验证中间件

4. **WebSocket** (`src/socketio/`, `src/websocket_chat.rs`)
   - 从 actix-ws 迁移到 axum::extract::ws
   - 更新 Socket.IO 实现

5. **数据库迁移** (`migrations/sqlite/`)
   - 转换剩余的 PostgreSQL 迁移 (002-010) 到 SQLite

## 技术要点 (Technical Notes)

### PostgreSQL → SQLite 转换规则

```
JSONB → TEXT (存储为 JSON 字符串)
BOOLEAN → INTEGER (0/1)
VARCHAR(n) → TEXT
BIGINT → INTEGER  
DATE → TEXT
$1, $2, ... → ?, ?, ...
```

### actix-web → axum 转换规则

```rust
// 状态访问
web::Data<AppState> → State(Arc<AppState>)

// 提取器
web::Json(payload) → Json(payload): Json<T>
web::Path(id) → Path(id): Path<String>

// 响应
HttpResponse::Ok().json(data) → Json(data)
```

## 环境变量 (Environment Variables)

```bash
# 本地 SQLite
DATABASE_URL=./data/webui.db

# 远程 Turso
DATABASE_URL=libsql://your-db.turso.io
TURSO_AUTH_TOKEN=your_auth_token
```

## 下一步行动 (Next Steps)

如果需要继续这个迁移：

1. 首先完成核心服务迁移（user, auth, config）
2. 然后迁移主要的路由处理器
3. 更新中间件系统
4. 最后处理 WebSocket 和 Socket.IO

每一步都应该：
- 更新代码
- 运行 `cargo check` 检查编译错误
- 修复错误
- 测试功能
- 提交更改

## 文件结构 (File Structure)

```
rust-backend/
├── src/
│   ├── main.rs           # 当前 actix-web 版本
│   ├── main_axum.rs      # 新的 axum 示例模板
│   ├── db.rs             # ✅ 已更新为 libSQL
│   ├── services/         # ⚠️ 需要更新 (~25 文件)
│   ├── routes/           # ⚠️ 需要更新 (~20 文件)
│   ├── middleware/       # ⚠️ 需要更新 (~3 文件)
│   └── ...
├── migrations/
│   ├── postgres/         # 原始迁移
│   └── sqlite/           # ✅ 新的 SQLite 迁移
├── Cargo.toml            # ✅ 已更新依赖
├── MIGRATION_GUIDE.md    # ✅ 详细迁移指南
└── MIGRATION_STATUS.md   # ✅ 本文件
```

## 预期收益 (Expected Benefits)

- 🚀 更现代和人性化的 API（axum）
- 🔧 更好的类型安全和提取器系统
- 🌐 分布式数据库能力（Turso）
- 💰 降低基础设施成本（Turso 免费额度慷慨）
- 📦 简化的中间件组合（tower）

## 参考资料 (References)

- [Axum 文档](https://docs.rs/axum/)
- [Tower 文档](https://docs.rs/tower/)
- [libSQL 文档](https://docs.turso.tech/libsql)
- [Turso 文档](https://docs.turso.tech/)
