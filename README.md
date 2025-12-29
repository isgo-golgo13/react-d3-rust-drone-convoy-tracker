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




## 0penCV Dependencies and Features of Tracking System Install

**Non-Linux (Unix)**
brew install opencv llvm

**Linux (Ubuntu)**  
sudo apt-get install libopencv-dev clang libclang-dev

**Or** Skip OpenCV commenting it out in Cargo.toml




### Test without (Rust) Server-Side
```shell
cd drone-convoy-sortie
npm install
npm run dev
# Shows 🟡 SIM - works exactly as before
```

### Test with (Rust) Server-Side
```shell
# Terminal 1
make dev-infra

# Terminal 2  
cd drone-convoy-tracking-server && cargo run --bin drone-api

# Terminal 3
cd drone-convoy-sortie && npm install && npm run dev
# Shows 🟢 LIVE - real-time updates from Rust
```



## Frontend-Backend React to Rust Server-Side Integration Workflow

The following graphic shows the React.js w/ D3.js and Google Maps API for React connected to the Rust server-side tracking system using WebSockets. The Rust server-side use of Redis Cache Server and ScyllaDB DB Cluster are **NOT** shown here.

### Functional Workflow

```
App starts
    │
    ▼
Check backend health (GET /health)
    │
    ├── Backend OK ──────────────────────┐
    │                                       ▼
    │                              Fetch drones from API
    │                              Connect WebSocket
    │                              Mode: 🟢 LIVE
    │                                       │
    │                              Real-time updates via WS
    │
    └── [X] Backend unavailable ────────────┐
                                           ▼
                                  Use INITIAL_DRONES from seedData
                                  Start local simulation
                                  Mode: 🟡 SIMULATION
                                           │
                                  Same behavior as before
```

## UI Changes

New connection status indicator in header:

```
┌─────────────────────────────────────────────────────────────────┐
│ DRONE CONVOY SORTIE          [🟢 LIVE][WS:ON][LIVE][SIM] ● ... │
└─────────────────────────────────────────────────────────────────┘
```

- **🟢 LIVE** - Connected to Rust backend, receiving real-time updates
- **🟡 SIM** - Using local simulation (backend unavailable)
- **WS: ON/OFF** - WebSocket connection status
- **[LIVE]** button - Switch to backend mode
- **[SIM]** button - Switch to simulation mode



## Testing

### Test Simulation Mode (No Backend)
```bash
cd drone-convoy-sortie
npm install
npm run dev
# Open http://localhost:5173
# Should show 🟡 SIM mode
# Drones should animate as before
```

### Test Live Mode (With Backend)
```bash
# Terminal 1: Start infrastructure
cd react-d3-drone-convoy-dash
make dev-infra

# Terminal 2: Start Rust backend
cd drone-convoy-tracking-server
cargo run --bin drone-api

# Terminal 3: Start React frontend
cd drone-convoy-sortie
npm run dev

# Open http://localhost:5173
# Should show 🟢 LIVE mode
# Drones receive real-time updates from backend
```



## Drone Data Shape

Same format your components already expect:

```javascript
{
  id: 'REAPER-01',
  callsign: 'Predator Alpha',
  currentWaypoint: 2,
  progress: 0.45,
  status: 'online',      // 'online' | 'offline' | 'warning'
  battery: 87,
  fuel: 92,
  altitude: 2500,
  speed: 135,
  systemHealth: 98,
  armament: ['Hellfire AGM-114', 'GBU-12'],
  lastUpdate: Date,
  lat: 34.5693,          // For map positioning
  lng: 69.2215,
}
```


## Drone Tracking ScyllaDB CQL Tables

The following are the ScyllaDB CQL Schema Tables for the tracking service Rust server connects/reads/writes to through Redis Cache.

- **drone_telemetry** -  time-series position data (empty - simulation doesn't write here)
- **drone_registry** - drone master list (has 3 sample drones)
- **missions** - mission config (has 1 sample mission)
- **waypoint_events** - waypoint arrivals (empty)
- **cv_tracking** - OpenCV data (empty)
- **alerts** - system alerts (empty)
- **p2p_metrics** - network metrics (empty)

The following CQL tables are provided for view executing the following Docker exec command.

```shell
docker exec -it scylla-node1 cqlsh -e "USE drone_convoy; DESCRIBE TABLES;"
```
The CQL shell command shows.
```shell
alerts       drone_registry   missions     waypoint_events
cv_tracking  drone_telemetry  p2p_metrics
```





## Runtime Resolution Tracking

### Stuck on "CONNECTING"
- Check backend is running: `curl http://localhost:3000/health`
- Check `.env.local` has correct URLs

### Always shows "SIM" even with backend running
- Check CORS on backend allows `http://localhost:5173`
- Check browser console for errors

### WebSocket disconnects frequently
- Backend may be crashing - check logs: `cargo run --bin drone-api`
- Network issues - check firewall isn't blocking port 9090

### Drones don't move in LIVE mode
- Backend simulation runs by default
- Check WebSocket messages in DevTools → Network → WS tab



## Future Extensions 


- Add DB writes to persist telemetry
- Add Redis for caching/pub-sub
- Enable OpenCV on Linux VM 
- Leptos frontend rewrite (100% Rust)