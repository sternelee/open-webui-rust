# Open WebUI with Rust Backend ｜ [简体中文](./README.zh-CN.md)

High‑Performance Rust Implementation of Open WebUI

<p align="center">
   <a href="https://youtu.be/xAPVZR_2nFk" target="_blank" rel="noopener noreferrer">
      <img width="600" src="./img/video-cover.png" alt="Open WebUI Backend in Rust">
   </a>
</p>

## Docker Quick Start

```
git clone https://github.com/knoxchat/open-webui-rust.git && cd open-webui-rust
docker compose up -d
```
> Ensure Docker and Docker Compose are ready

## Overview

The Rust backend is a drop-in replacement for the Python backend, offering:

- **10-50x faster response times** for API endpoints
- **70% lower memory usage** under load
- **Native concurrency** with Tokio's async runtime
- **Type safety** preventing entire classes of runtime errors
- **Zero-copy streaming** for chat completions
- **Production-ready** with comprehensive error handling

## **IMPORTANT‼️** Your sponsorship will accelerate and improve the project development progress:

- **Please Scan the QR Code Below via Alipay or Paypal to Sponsor**
- **Contact us: support@knox.chat**

<p align="center" style="display: flex; align-items: center; justify-content: center; gap: 20px;">
   <img width="246" src="./img/ali.png" alt="Name">
   <img width="229" src="./img/paypal.png" alt="Name">
</p>

## Sponsor List

| Name | Sponsorship Amount | Contributed Files | Privileges |
|------|----------|---------|---------|
| [![Baitian Medical](./img/baitian.png)](https://baitianjituan.com) | ¥5000 | 300 | Dedicated Technical Support |
| 孔祥康 | ¥500 | 30 | Email/IM Service |
| Knox User Anonymous Sponsorship | ¥300 | 18 | Email/IM Service |
| [Bestming](https://www.mingagent.com) | ¥100 | 6 | Email/IM Service |
| HJPING | ¥100 | 6 | Email/IM Service |
| KingZ | ¥50 | 2 | Email Service |
| JimLi | ¥66 | 2 | Email Service |
| shanwu | ¥50 | 2 | Email Service |
| xixi | ¥50 | 2 | Email Service |

## Table of Contents

- [Architecture](#architecture)
- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Configuration](#configuration)
- [Running the Server](#running-the-server)
- [API Compatibility](#api-compatibility)
- [Performance](#performance)
- [Development](#development)
- [Testing](#testing)
- [Deployment](#deployment)

## Architecture

### Technology Stack

- **Framework**: Actix-Web 4.x (one of the fastest web frameworks)
- **Runtime**: Tokio (async/await native runtime)
- **Database**: PostgreSQL with SQLx (compile-time checked queries)
- **Caching**: Redis with deadpool connection pooling
- **Authentication**: JWT with jsonwebtoken + Argon2/Bcrypt
- **Serialization**: Serde (zero-copy deserialization)
- **HTTP Client**: Reqwest (async HTTP/2 client)

### Project Structure

```
rust-backend/
├── src/
│   ├── main.rs              # Application entry point
│   ├── config.rs            # Configuration management
│   ├── db.rs                # Database connection pooling
│   ├── error.rs             # Centralized error handling
│   ├── models/              # Data models (25+ entities)
│   │   ├── auth.rs          # User, session, API key models
│   │   ├── chat.rs          # Chat, message models
│   │   ├── model.rs         # AI model configurations
│   │   └── ...              # Channel, file, knowledge, etc.
│   ├── routes/              # HTTP route handlers (25+ modules)
│   │   ├── auth.rs          # Authentication endpoints
│   │   ├── chats.rs         # Chat management
│   │   ├── openai.rs        # OpenAI-compatible API
│   │   └── ...              # Audio, images, tools, etc.
│   ├── services/            # Business logic layer (27+ services)
│   │   ├── chat.rs          # Chat processing service
│   │   ├── auth.rs          # Authentication service
│   │   ├── rag.rs           # RAG (Retrieval) service
│   │   └── ...              # Model, user, file services
│   ├── middleware/          # Request/response middleware
│   │   ├── auth.rs          # JWT authentication
│   │   ├── audit.rs         # Request auditing
│   │   └── rate_limit.rs    # Rate limiting
│   ├── utils/               # Utility functions
│   │   ├── auth.rs          # JWT helpers
│   │   ├── embeddings.rs    # Vector embeddings
│   │   └── chat_completion.rs # Chat utilities
│   ├── socket.rs            # WebSocket/Socket.IO support
│   └── websocket_chat.rs    # Real-time chat streaming
├── migrations/              # Database migrations
│   └── postgres/            # PostgreSQL schema migrations
├── Cargo.toml               # Rust dependencies
└── .env.example             # Environment configuration template
```

## Prerequisites

- **Rust**: 1.75+ (install via [rustup](https://rustup.rs/))
- **PostgreSQL**: 13+ (required)
- **Redis**: 6.0+ (optional, recommended for sessions and caching)
- **Operating System**: Linux, macOS, or Windows

## Installation

### 1. Clone Repository

```bash
cd rust-backend
```

### 2. Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### 3. Set Up Database

```bash
# Create PostgreSQL database
createdb openwebui

# Set database URL
export DATABASE_URL="postgresql://postgres:password@localhost:5432/openwebui"
```

### 4. Install Dependencies

```bash
# Dependencies are automatically managed by Cargo
cargo fetch
```

## Configuration

### Development with Docker Compose

For local development with all services (PostgreSQL, Redis, ChromaDB):

```bash
# Start all development services
docker compose -f docker-compose.dev.yaml up -d

# View logs
docker compose -f docker-compose.dev.yaml logs -f

# Stop services
docker compose -f docker-compose.dev.yaml down

# Stop services and remove volumes (clean slate)
docker compose -f docker-compose.dev.yaml down -v
```

**Services included:**
- **PostgreSQL** (port 5432): Main database
- **Redis** (port 6379): Caching and session management
- **ChromaDB** (port 8000): Vector database for RAG/embeddings
- **pgAdmin** (port 5050): PostgreSQL admin UI (optional, use `--profile tools`)
- **Redis Commander** (port 8082): Redis admin UI (optional, use `--profile tools`)

### Rust Backend Environment Variables

Create `.env` file in `rust-backend/` directory:

See `rust-backend/env.example` for complete configuration options.

## Running the Server

### Development Mode with Docker Services

**Step 1: Start development services**

```bash
# Start PostgreSQL, Redis, and ChromaDB
docker compose -f docker-compose.dev.yaml up -d

# Verify services are running
docker compose -f docker-compose.dev.yaml ps
```

**Step 2: Configure Rust backend**

```bash
cd rust-backend

# Copy example environment file
cp env.example .env

# Edit .env and set WEBUI_SECRET_KEY to a fixed value
# Example: WEBUI_SECRET_KEY=$(uuidgen | tr '[:upper:]' '[:lower:]')
nano .env
```

**Step 3: Run Rust backend**

```bash
cargo run
```

The server will start at `http://0.0.0.0:8080`

**Benefits of this setup:**
- All dependencies (PostgreSQL, Redis, ChromaDB) run in Docker
- Rust backend runs natively for faster compilation and debugging
- JWT tokens persist across backend restarts (with fixed WEBUI_SECRET_KEY)
- Easy to reset database with `docker compose down -v`

## Performance

### Quick Summary

| Metric | Python (FastAPI) | Rust (Actix-Web) | Improvement |
|--------|------------------|------------------|-------------|
| Login (p50) | 45ms | 3ms | **15x faster** |
| Chat Completion (p50) | 890ms | 35ms* | **25x faster** |
| Model List (p50) | 23ms | 1.2ms | **19x faster** |
| Memory (1000 req) | 450 MB | 85 MB | **5.3x lower** |
| Throughput | 850 req/s | 12,400 req/s | **14.6x higher** |

*Note: Chat completion speed primarily depends on LLM provider. Rust excels at streaming and handling overhead.

## Development

### Prerequisites

```bash
# Install development tools
rustup component add rustfmt clippy

# Install cargo-watch for auto-reload
cargo install cargo-watch
```

### Development Workflow

```bash
# Ensure Docker services are running
docker compose -f docker-compose.dev.yaml up -d

# Auto-reload on file changes
cd rust-backend
cargo watch -x run

# Run tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Format code
cargo fmt

# Lint code
cargo clippy -- -D warnings

# Check without building
cargo check
```

## Testing

### Unit Tests

```bash
cargo test --lib
```

### Integration Tests

```bash
# Set test database
export DATABASE_URL=postgresql://postgres:postgres@localhost:5432/openwebui_test

# Run integration tests
cargo test --test '*'
```

### Load Testing

```bash
# Install wrk
brew install wrk  # macOS
sudo apt install wrk  # Ubuntu

# Run load test
wrk -t4 -c100 -d30s --latency http://localhost:8080/health
```

## Deployment

### Building for Production

```bash
# Build optimized binary
cargo build --release

# Binary location
./target/release/open-webui-rust

# Strip symbols (reduces size)
strip ./target/release/open-webui-rust
```
