# 🚁 Drone Convoy Tracking System

A full-stack tactical drone convoy management system featuring a React + D3.js + Google Maps frontend with a Rust microservices backend, OpenCV computer vision, and ScyllaDB persistence.

## 🚀 Quick Start

### Prerequisites
- Node.js 18+ and npm
- Rust 1.75+ (for local development)
- Docker and Docker Compose
- Google Maps API Key

### Option 1: Full Stack with Docker Compose (Recommended)

```bash
# 1. Clone the repository
git clone https://github.com/isgo-golgo13/react-d3-drone-convoy-dash.git
cd react-d3-drone-convoy-dash

# 2. Set your Google Maps API key
echo "VITE_GOOGLE_MAPS_API_KEY=your-api-key-here" > drone-convoy-sortie/.env

# 3. Start everything
cd drone-convoy-tracking-server
make docker-up

# 4. Wait ~60 seconds for ScyllaDB, then check status
make docker-status
```

Access the dashboard at **http://localhost:8080**

### Option 2: Development Mode (Frontend + Backend separately)

**Terminal 1 - React Frontend:**
```bash
cd drone-convoy-sortie
npm install
npm run dev
# Runs at http://localhost:5173
```

**Terminal 2 - Rust Backend:**
```bash
cd drone-convoy-tracking-server
make docker-deps    # Start ScyllaDB, Redis, Prometheus, etc.
make dev-backend    # Start Rust API server
# API at http://localhost:3000, WebSocket at ws://localhost:9090
```

## Service URLs

| Service | URL | Description |
|---------|-----|-------------|
| React Dashboard | http://localhost:8080 | Main tactical UI |
| REST API | http://localhost:3000 | Drone/mission endpoints |
| WebSocket | ws://localhost:9090 | Real-time telemetry |
| Grafana | http://localhost:3001 | Dashboards (admin/admin) |
| Prometheus | http://localhost:9091 | Metrics |
| Jaeger | http://localhost:16686 | Tracing |

## Features

### Frontend
- Google Maps satellite view of Afghanistan convoy route
- 12 military REAPER drones with real-time tracking
- 12 strategic waypoints (Base Alpha → Terminal Lima)
- Live telemetry: battery, fuel, altitude, speed, heading
- Neumorphic control panel with military aesthetics
- Glitch effects and tactical styling

### Server-Side (Rust)  
- Rust microservices with Axum REST API
- WebSocket streaming for real-time updates
- OpenCV red halo detection with Kalman filtering
- ScyllaDB 3-node cluster for time-series data
- Prometheus metrics + Grafana dashboards
- libp2p mesh networking (optional)

## Connecting Frontend to Server-Side

The server-side broadcasts drone updates through WebSocket. Connect your React frontend:

```javascript
// In your React component
useEffect(() => {
  const ws = new WebSocket('ws://localhost:9090');
  
  ws.onmessage = (event) => {
    const msg = JSON.parse(event.data);
    if (msg.type === 'Event') {
      const { event_type, payload } = msg.payload;
      // Handle: DRONE_POSITION_UPDATED, WAYPOINT_REACHED, ...
    }
  };
  
  return () => ws.close();
}, []);
```

## Make Commands

```bash
make build          # Build frontend + backend
make docker-up      # Start all services
make docker-down    # Stop all services
make docker-status  # Show service URLs
make test           # Run all tests
make lint           # Lint code
make db-shell       # Open ScyllaDB CQL shell
```

## Project Structure

```
react-d3-drone-convoy-tracker/
├── drone-convoy-sortie/           # React Frontend
│   ├── src/components/            # TacticalMap, DroneStatus, etc.
│   └── package.json
└── drone-convoy-tracking-server/  # Rust Backend
    ├── Cargo.toml                 # Workspace
    ├── docker-compose.yaml
    ├── Makefile
    └── crates/
        ├── drone-core/            # Shared models
        ├── drone-cv/              # OpenCV tracking
        ├── drone-db/              # ScyllaDB
        ├── drone-api/             # REST API
        ├── drone-websocket/       # Real-time
        ├── drone-telemetry/       # Metrics
        ├── drone-p2p/             # Mesh network
        └── drone-tracker/         # Orchestration
```

## License

MIT
