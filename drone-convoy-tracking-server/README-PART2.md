# Drone Convoy Tracking Server - Part 2 of 3

## 🚁 Overview

This is Part 2 of the Rust backend. It contains the API server, WebSocket server, and metrics:

- **drone-api**: Axum REST API server (main entry point)
- **drone-websocket**: Real-time WebSocket server for React frontend
- **drone-telemetry**: Prometheus metrics exporter

## Contents

```
crates/
├── drone-api/                 # Main API Server
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs            # Entry point with simulation
│       ├── config.rs          # Configuration management
│       ├── state.rs           # Application state
│       ├── routes.rs          # API route definitions
│       ├── handlers.rs        # Request handlers
│       └── error.rs           # Error types
│
├── drone-websocket/           # WebSocket Server
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs             # WebSocket server & handlers
│       ├── hub.rs             # Connection hub & broadcasting
│       └── error.rs           # Error types
│
└── drone-telemetry/           # Metrics
    ├── Cargo.toml
    └── src/
        └── lib.rs             # Prometheus metrics collector
```

## API Endpoints

### Health & Status
- `GET /health` - Health check
- `GET /ready` - Readiness probe (Kubernetes)
- `GET /status` - System status overview
- `GET /metrics` - Prometheus metrics

### Drones
- `GET /api/v1/drones` - List all drones
- `GET /api/v1/drones/:id` - Get drone by ID
- `GET /api/v1/drones/:id/telemetry` - Get drone telemetry
- `GET /api/v1/drones/:id/position` - Get drone position
- `POST /api/v1/drones/:id/command` - Send command to drone

### Mission
- `GET /api/v1/mission` - Get active mission
- `POST /api/v1/mission/start` - Start mission
- `POST /api/v1/mission/pause` - Pause mission
- `POST /api/v1/mission/resume` - Resume mission
- `POST /api/v1/mission/abort` - Abort mission
- `GET /api/v1/mission/waypoints` - Get waypoints

### CV Tracking
- `GET /api/v1/tracking` - Get tracking results
- `GET /api/v1/tracking/stats` - Get tracking statistics

### WebSocket
- `GET /api/v1/ws/info` - WebSocket connection info
- `ws://localhost:9090` - WebSocket endpoint

### State
- `GET /api/v1/state` - Full state snapshot for frontend

## WebSocket Protocol

Connect to `ws://localhost:9090` to receive real-time updates.

### Server → Client Messages
```json
{
  "type": "Event",
  "payload": {
    "id": "uuid",
    "timestamp": "2024-01-01T00:00:00Z",
    "event_type": "DRONE_POSITION_UPDATED",
    "payload": {
      "type": "DronePosition",
      "data": {
        "drone_id": "REAPER-01",
        "position": { "latitude": 34.5553, "longitude": 69.2075, "altitude": 3000 },
        "telemetry": { ... }
      }
    }
  }
}
```

### Client → Server Messages
```json
{
  "type": "Subscribe",
  "payload": { "drone_ids": ["REAPER-01", "REAPER-02"] }
}
```

## Prometheus Metrics

Available at `/metrics`:
- `drone_convoy_drones_total` - Total drone count
- `drone_convoy_drone_battery_percent` - Battery levels
- `drone_convoy_ws_connections` - WebSocket connections
- `drone_convoy_cv_tracks_active` - Active CV tracks
- `drone_convoy_api_requests_total` - API request counts

## Part 3 Will Include

- `drone-p2p`: libp2p mesh networking between drones
- `drone-tracker`: Main tracking orchestration logic
- Complete Grafana dashboards
- Integration tests

## ⚡ After All Parts

Merge into your repo:
```bash
unzip drone-convoy-part-2.zip
# Files go into crates/drone-api, crates/drone-websocket, crates/drone-telemetry
```

Then run:
```bash
make docker-up
# API: http://localhost:3000
# WebSocket: ws://localhost:9090
# Frontend: http://localhost:8080
```
