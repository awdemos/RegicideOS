# Candle Inference Server

High-performance ML inference server built with Rust and Candle for RegicideOS AI layer.

## Features

- Rust-native performance with Candle ML framework
- Multiple model support (LLMs, embeddings, code models)
- RESTful API with /health, /models, /inference endpoints
- Security-first design with systemd hardening

## Quick Start

```bash
cargo build --release
./target/release/candle-server
```

## API Endpoints

```bash
# Health check
GET /health

# List models  
GET /models

# Run inference
POST /inference
{
  "prompt": "Explain BTRFS",
  "max_tokens": 100,
  "temperature": 0.7
}
```

Part of the RegicideOS AI layer infrastructure.
