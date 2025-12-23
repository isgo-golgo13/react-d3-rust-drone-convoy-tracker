# Drone Attack Convoy Tracker
Drone Attack Tracking Convoy Geo-Grid Service using React.js with D3.j, Google Maps for React, OpenCV Tracking API and Rust. 


![drone-convoy-screen](docs/react-d3-convoy-screen.png)




## Drone Attack Convoy Dash Service Overview

- 12 waypoints in Afghanistan region with GD Icons-style markers
- 4 mock attack drones with real-time simulation
- Google Maps integration (needs API key)
- Real-time status monitoring with battery, altitude, waypoint progress
- Tactical-style dark UI with military aesthetics

For phase two, the service uses OpenCV Rust API (for Drone halo tracking) and Rust Tokio Async for the server-side. Following is the  architecture.

```shell
// Cargo.toml dependencies
[dependencies]
opencv = "0.88"
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
warp = "0.3"
uuid = { version = "1.0", features = ["v4"] }
```

### (1) Drone Tracking System (Rust, OpenCV)

- Real-time object detection for each drone
- Unique ID halo rendering around detected drones
- Position tracking with Kalman filtering
- Collision avoidance calculations


### (2) P2P Communication Network

- Each drone maintains peer connections
- Real-time position/status broadcasting
- Swarm coordination algorithms


### (3) Integration Points

- WebSocket connection to React frontend
- Real-time video feed processing
- GPS coordinate mapping to pixel coordinates


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



## Creating the Front-End React Project

```shell
# 1. Create project
npm create vite@latest drone-convoy-sortie -- --template react
cd drone-convoy-sortie

# 2. Install dependencies  
npm install d3 lucide-react
npm install -D postcss autoprefixer @types/d3

# 3. Copy all provided code files
# 4. Start development
npm run dev  # http://localhost:5173

# Or use Docker
make quick-start  # http://localhost:8080
```
