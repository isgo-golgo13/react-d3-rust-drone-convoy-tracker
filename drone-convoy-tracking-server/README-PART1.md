# Drone Convoy Tracking Server - Part 1 of 3

## 🚁 Overview

This is Part 1 of the Rust backend for the Drone Convoy Tracking System. It contains:

- **Workspace Configuration**: Top-level `Cargo.toml` with all dependencies
- **Core Crate**: Shared domain models (Drone, Mission, Waypoint, Telemetry)
- **CV Crate**: OpenCV integration for red halo detection and tracking
- **DB Crate**: ScyllaDB integration for persistence
- **Infrastructure**: Docker Compose, Makefile, monitoring configs

## 📦 Contents

```
drone-convoy-tracking-server/
├── Cargo.toml                 # Workspace manifest
├── Makefile                   # Build automation
├── Dockerfile                 # Multi-stage with OpenCV
├── docker-compose.yaml        # Full stack orchestration
├── schema.cql                 # ScyllaDB schema
├── crates/
│   ├── drone-core/            # Shared models
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs         # Domain models
│   │       ├── error.rs       # Error types
│   │       ├── geo.rs         # Geographic calculations
│   │       └── events.rs      # Event types
│   ├── drone-cv/              # OpenCV tracking
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs         # CV engine
│   │       ├── config.rs      # Configuration
│   │       ├── error.rs       # Error types
│   │       ├── detector.rs    # Halo detection
│   │       ├── kalman.rs      # Kalman filtering
│   │       ├── tracker.rs     # Multi-object tracking
│   │       └── renderer.rs    # Overlay rendering
│   └── drone-db/              # ScyllaDB persistence
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs         # Repositories
│           ├── error.rs       # Error types
│           ├── repository.rs  # Re-exports
│           └── migrations.rs  # Schema migrations
└── monitoring/
    ├── prometheus.yml         # Prometheus config
    └── grafana/
        └── provisioning/      # Grafana setup
```

## 🔧 Part 2 Will Include

- `drone-api`: Axum REST API server
- `drone-websocket`: Real-time WebSocket server
- `drone-telemetry`: Prometheus metrics

## 🔧 Part 3 Will Include

- `drone-p2p`: libp2p mesh networking
- `drone-tracker`: Main tracking orchestration
- Complete Grafana dashboards
- Integration tests

## ⚡ Quick Start (After All Parts)

```bash
# Build everything
make build

# Start with Docker Compose
make docker-up

# Check status
make docker-status
```

## 🎯 Key Features

### OpenCV Red Halo Tracking
- Hough Circle Transform for halo detection
- Kalman filtering for smooth position prediction
- Multi-object tracking with unique IDs
- Geo-coordinate projection from camera view

### ScyllaDB Integration
- 3-node cluster for high availability
- Time-series telemetry storage with TTL
- Waypoint event recording
- CV tracking result persistence
