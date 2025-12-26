/**
 * useBackendConnection Hook
 * 
 * Main integration hook for Drone Convoy system.
 * 
 * Features:
 * - Connects to Rust WebSocket for real-time drone telemetry
 * - Fetches initial data from REST API
 * - Falls back to local simulation when backend unavailable
 * - Maintains same data shape as existing components expect
 */

import { useState, useEffect, useCallback, useRef } from 'react';
import { WAYPOINTS, INITIAL_DRONES } from '../data/seedData.js';

const API_URL = import.meta.env.VITE_API_URL || 'http://localhost:3000';
const WS_URL = import.meta.env.VITE_WS_URL || 'ws://localhost:9090';

const RECONNECT_DELAY = 3000;
const MAX_RECONNECT_ATTEMPTS = 10;
const SIMULATION_INTERVAL = 100; // ms

/**
 * Calculate which waypoint segment a position is on
 */
function calculateWaypointFromPosition(lat, lng) {
  let closestWaypoint = 0;
  let closestDistance = Infinity;
  
  // Find closest waypoint
  for (let i = 0; i < WAYPOINTS.length; i++) {
    const wp = WAYPOINTS[i];
    const dist = Math.sqrt(
      Math.pow(lat - wp.lat, 2) + Math.pow(lng - wp.lng, 2)
    );
    if (dist < closestDistance) {
      closestDistance = dist;
      closestWaypoint = i;
    }
  }
  
  // Calculate progress between waypoints
  if (closestWaypoint < WAYPOINTS.length - 1) {
    const currentWP = WAYPOINTS[closestWaypoint];
    const nextWP = WAYPOINTS[closestWaypoint + 1];
    
    const totalDist = Math.sqrt(
      Math.pow(nextWP.lat - currentWP.lat, 2) + 
      Math.pow(nextWP.lng - currentWP.lng, 2)
    );
    
    const currentDist = Math.sqrt(
      Math.pow(lat - currentWP.lat, 2) + 
      Math.pow(lng - currentWP.lng, 2)
    );
    
    const progress = Math.min(1, currentDist / totalDist);
    
    return { waypoint: closestWaypoint, progress };
  }
  
  return { waypoint: closestWaypoint, progress: 1 };
}




/**
 * Connection modes
 */
export const ConnectionMode = {
  CONNECTING: 'connecting',
  LIVE: 'live',
  SIMULATION: 'simulation',
  ERROR: 'error',
};

/**
 * Transform backend drone to frontend format
 */
function transformBackendDrone(backendDrone) {
  return {
    id: backendDrone.id || backendDrone.drone_id,
    callsign: backendDrone.callsign || backendDrone.name || backendDrone.id,
    currentWaypoint: backendDrone.waypoint_index ?? backendDrone.currentWaypoint ?? 0,
    progress: backendDrone.waypoint_progress ?? backendDrone.progress ?? 0,
    status: backendDrone.status?.toLowerCase() || 'online',
    battery: backendDrone.telemetry?.battery_level ?? backendDrone.battery ?? 100,
    fuel: backendDrone.telemetry?.fuel_level ?? backendDrone.fuel ?? 100,
    altitude: backendDrone.position?.altitude ?? backendDrone.altitude ?? 2500,
    speed: backendDrone.telemetry?.speed ?? backendDrone.speed ?? 135,
    systemHealth: backendDrone.telemetry?.system_health ?? backendDrone.systemHealth ?? 95,
    armament: backendDrone.armament || ['Hellfire AGM-114'],
    lastUpdate: backendDrone.last_update ? new Date(backendDrone.last_update) : new Date(),
    lat: backendDrone.position?.latitude ?? backendDrone.lat ?? 0,
    lng: backendDrone.position?.longitude ?? backendDrone.lng ?? 0,
  };
}

export function useBackendConnection() {
  // Connection state
  const [mode, setMode] = useState(ConnectionMode.CONNECTING);
  const [isConnected, setIsConnected] = useState(false);
  const [error, setError] = useState(null);

  // Data state
  const [drones, setDrones] = useState([]);
  const [isSimulating, setIsSimulating] = useState(false);
  const [simulationSpeed, setSimulationSpeed] = useState(1);

  // Refs
  const wsRef = useRef(null);
  const reconnectAttempts = useRef(0);
  const reconnectTimeout = useRef(null);
  const simulationInterval = useRef(null);

  // Debug: watch drone state changes
  useEffect(() => {
    if (drones.length > 0 && mode === ConnectionMode.LIVE) {
      console.log('DRONES STATE:', drones[0].id, 'lat:', drones[0].lat, 'lng:', drones[0].lng);
    }
  }, [drones, mode]);

  /**
   * Initialize drones for simulation mode
   */
  const initializeSimulationDrones = useCallback(() => {
    if (INITIAL_DRONES && INITIAL_DRONES.length > 0) {
      return INITIAL_DRONES.map(drone => ({
        ...drone,
        lastUpdate: new Date(),
      }));
    }

    const generatedDrones = [];
    for (let i = 1; i <= 12; i++) {
      generatedDrones.push({
        id: `REAPER-${i.toString().padStart(2, '0')}`,
        callsign: `REAPER-${i.toString().padStart(2, '0')}`,
        currentWaypoint: 0,
        progress: Math.random() * 0.3,
        status: i <= 10 ? 'online' : 'offline',
        battery: 75 + Math.random() * 25,
        fuel: 60 + Math.random() * 40,
        altitude: 140 + Math.random() * 20,
        speed: 45 + Math.random() * 15,
        systemHealth: 85 + Math.random() * 15,
        armament: Math.floor(Math.random() * 8) + 4,
        lastUpdate: new Date(),
        lat: 0,
        lng: 0
      });
    }
    return generatedDrones;
  }, []);

  /**
   * Check backend health
   */
  const checkBackendHealth = useCallback(async () => {
    try {
      const response = await fetch(`${API_URL}/health`, { 
        method: 'GET',
        signal: AbortSignal.timeout(3000)
      });
      return response.ok;
    } catch {
      return false;
    }
  }, []);

  /**
   * Fetch drones from backend
   */
  const fetchDrones = useCallback(async () => {
    try {
      const response = await fetch(`${API_URL}/api/v1/drones`);
      if (!response.ok) throw new Error('Failed to fetch drones');
      const data = await response.json();
      
      if (data.drones && Array.isArray(data.drones)) {
        return data.drones.map(transformBackendDrone);
      }
      return null;
    } catch (err) {
      console.warn('Failed to fetch drones:', err.message);
      return null;
    }
  }, []);

  /**
   * Handle WebSocket message
   */
  const handleWebSocketMessage = useCallback((event) => {
  try {
    const data = typeof event === 'string' ? JSON.parse(event) : 
                 event.data ? JSON.parse(event.data) : event;
    
    // Skip non-Event messages (like InitialState)
    if (data.type !== 'Event') return;
    
    const eventPayload = data.payload;
    
    if (eventPayload.event_type === 'DRONE_POSITION_UPDATED') {
      const droneData = eventPayload.payload.data;
      const droneId = droneData.drone_id;

      setDrones(prev => {
        const newDrones = prev.map(drone => {
          if (drone.id !== droneId) return drone;

          const newLat = droneData.position.latitude;
          const newLng = droneData.position.longitude;
          
          // Calculate waypoint progress from position
          const waypointInfo = calculateWaypointFromPosition(newLat, newLng);

          return {
            ...drone,
            lat: newLat,
            lng: newLng,
            altitude: droneData.position.altitude,
            battery: droneData.telemetry.battery_level,
            fuel: droneData.telemetry.fuel_level,
            speed: droneData.telemetry.speed,
            systemHealth: droneData.telemetry.system_health,
            currentWaypoint: waypointInfo.waypoint,
            progress: waypointInfo.progress,
            lastUpdate: new Date(),
          };
        });
        return newDrones;
      });
    }
  } catch (err) {
    console.error('Failed to parse WebSocket message:', err);
  }
}, []);


  /**
   * Connect to WebSocket
   */
  const connectWebSocket = useCallback(() => {
    if (wsRef.current?.readyState === WebSocket.OPEN) return;

    try {
      console.log(`🔌 Connecting to WebSocket: ${WS_URL}`);
      const ws = new WebSocket(WS_URL);

      ws.onopen = () => {
        console.log('✅ WebSocket connected');
        setIsConnected(true);
        setError(null);
        reconnectAttempts.current = 0;
      };

      ws.onmessage = handleWebSocketMessage;

      ws.onerror = (event) => {
        console.error('WebSocket error:', event);
      };

      ws.onclose = (event) => {
        console.log(`🔌 WebSocket closed: ${event.code}`);
        setIsConnected(false);
        wsRef.current = null;

        if (mode === ConnectionMode.LIVE && reconnectAttempts.current < MAX_RECONNECT_ATTEMPTS) {
          reconnectAttempts.current += 1;
          console.log(`🔄 Reconnecting (${reconnectAttempts.current}/${MAX_RECONNECT_ATTEMPTS})...`);
          reconnectTimeout.current = setTimeout(connectWebSocket, RECONNECT_DELAY);
        }
      };

      wsRef.current = ws;
    } catch (err) {
      console.error('Failed to create WebSocket:', err);
      setError(err.message);
    }
  }, [handleWebSocketMessage, mode]);

  /**
   * Disconnect WebSocket
   */
  const disconnectWebSocket = useCallback(() => {
    if (reconnectTimeout.current) {
      clearTimeout(reconnectTimeout.current);
      reconnectTimeout.current = null;
    }
    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }
    setIsConnected(false);
  }, []);

  /**
   * Run simulation update
   */
  // const runSimulationUpdate = useCallback(() => {
  //   setDrones(prevDrones => 
  //     prevDrones.map(drone => {
  //       if (drone.status === 'offline') return drone;

  //       let newProgress = drone.progress + (0.01 * simulationSpeed);
  //       let newWaypoint = drone.currentWaypoint;

  //       if (newProgress >= 1) {
  //         newProgress = 0;
  //         newWaypoint = Math.min(drone.currentWaypoint + 1, WAYPOINTS.length - 1);
  //       }

  //       const batteryDrain = 0.1 * simulationSpeed;
  //       const fuelDrain = 0.15 * simulationSpeed;

  //       return {
  //         ...drone,
  //         progress: newProgress,
  //         currentWaypoint: newWaypoint,
  //         battery: Math.max(0, drone.battery - batteryDrain),
  //         fuel: Math.max(0, drone.fuel - fuelDrain),
  //         speed: 45 + Math.random() * 15,
  //         altitude: 140 + Math.random() * 20,
  //         systemHealth: Math.max(70, drone.systemHealth - Math.random() * 0.1),
  //         lastUpdate: new Date(),
  //       };
  //     })
  //   );
  // }, [simulationSpeed]);
  const runSimulationUpdate = useCallback(() => {
  setDrones(prevDrones => 
    prevDrones.map(drone => {
      if (drone.status === 'offline') return drone;

      let newProgress = drone.progress + (0.01 * simulationSpeed);
      let newWaypoint = drone.currentWaypoint;

      if (newProgress >= 1) {
        newProgress = 0;
        newWaypoint = Math.min(drone.currentWaypoint + 1, WAYPOINTS.length - 1);
      }

      // Calculate lat/lng from waypoints for SIM mode
      const currentWP = WAYPOINTS[newWaypoint];
      const nextWP = WAYPOINTS[Math.min(newWaypoint + 1, WAYPOINTS.length - 1)];
      const lat = currentWP.lat + (nextWP.lat - currentWP.lat) * newProgress;
      const lng = currentWP.lng + (nextWP.lng - currentWP.lng) * newProgress;

      const batteryDrain = 0.1 * simulationSpeed;
      const fuelDrain = 0.15 * simulationSpeed;

      return {
        ...drone,
        progress: newProgress,
        currentWaypoint: newWaypoint,
        lat,
        lng,
        battery: Math.max(0, drone.battery - batteryDrain),
        fuel: Math.max(0, drone.fuel - fuelDrain),
        speed: 45 + Math.random() * 15,
        altitude: 140 + Math.random() * 20,
        systemHealth: Math.max(70, drone.systemHealth - Math.random() * 0.1),
        lastUpdate: new Date(),
      };
    })
  );
}, [simulationSpeed]);



  /**
   * Simulation controls
   */
  const startSimulation = useCallback(() => {
    setIsSimulating(true);
  }, []);

  const stopSimulation = useCallback(() => {
    setIsSimulating(false);
  }, []);

  const toggleSimulation = useCallback(() => {
    setIsSimulating(prev => !prev);
  }, []);

  // const resetSimulation = useCallback(() => {
  //   setIsSimulating(false);
  //   setDrones(initializeSimulationDrones());
  // }, [initializeSimulationDrones]);


  const resetSimulation = useCallback(async () => {
  setIsSimulating(false);
  
  // Reset all drones to starting position
  setDrones(prev => prev.map(drone => ({
    ...drone,
    currentWaypoint: 0,
    progress: 0,
    lat: WAYPOINTS[0].lat,
    lng: WAYPOINTS[0].lng,
    battery: 100,
    fuel: 100,
    systemHealth: 95 + Math.random() * 5,
    status: drone.status === 'offline' ? 'offline' : 'online',
    lastUpdate: new Date(),
  })));

  // If in LIVE mode, also reset backend
  if (mode === ConnectionMode.LIVE) {
    try {
      await fetch(`${API_URL}/api/v1/mission/reset`, { method: 'POST' });
    } catch (err) {
      console.warn('Failed to reset backend simulation:', err);
    }
  }
}, [mode]);
  /**
   * Mode switching
   */
  const switchToLive = useCallback(async () => {
    setMode(ConnectionMode.CONNECTING);
    stopSimulation();

    const isHealthy = await checkBackendHealth();
    if (isHealthy) {
      const backendDrones = await fetchDrones();
      if (backendDrones) {
        setDrones(backendDrones);
      }
      setMode(ConnectionMode.LIVE);
      connectWebSocket();
    } else {
      setMode(ConnectionMode.SIMULATION);
      setError('Backend not available');
    }
  }, [checkBackendHealth, fetchDrones, connectWebSocket, stopSimulation]);

  const switchToSimulation = useCallback(() => {
    disconnectWebSocket();
    setMode(ConnectionMode.SIMULATION);
    setDrones(initializeSimulationDrones());
  }, [disconnectWebSocket, initializeSimulationDrones]);

  // Initialize on mount
  useEffect(() => {
    const initialize = async () => {
      console.log('🚀 Initializing backend connection...');
      
      const isHealthy = await checkBackendHealth();
      
      if (isHealthy) {
        console.log('✅ Backend is healthy, switching to LIVE mode');
        const backendDrones = await fetchDrones();
        if (backendDrones && backendDrones.length > 0) {
          setDrones(backendDrones);
        } else {
          // Initialize with empty drones, will be populated by WebSocket
          setDrones(initializeSimulationDrones());
        }
        setMode(ConnectionMode.LIVE);
        connectWebSocket();
      } else {
        console.log('⚠️ Backend unavailable, using SIMULATION mode');
        setDrones(initializeSimulationDrones());
        setMode(ConnectionMode.SIMULATION);
        setIsSimulating(true);
      }
    };

    initialize();

    return () => {
      disconnectWebSocket();
      if (simulationInterval.current) {
        clearInterval(simulationInterval.current);
      }
    };
  }, []);

  // Run simulation loop when simulating
  useEffect(() => {
    if (isSimulating && mode === ConnectionMode.SIMULATION) {
      simulationInterval.current = setInterval(runSimulationUpdate, SIMULATION_INTERVAL);
    } else {
      if (simulationInterval.current) {
        clearInterval(simulationInterval.current);
        simulationInterval.current = null;
      }
    }

    return () => {
      if (simulationInterval.current) {
        clearInterval(simulationInterval.current);
      }
    };
  }, [isSimulating, mode, runSimulationUpdate]);

  return {
    // Data
    drones,
    waypoints: WAYPOINTS,
    
    // Simulation controls
    isSimulating,
    simulationSpeed,
    startSimulation,
    stopSimulation,
    toggleSimulation,
    resetSimulation,
    setSimulationSpeed,
    
    // Connection status
    mode,
    isConnected,
    isLive: mode === ConnectionMode.LIVE,
    isSimulationMode: mode === ConnectionMode.SIMULATION,
    error,
    
    // Mode switching
    switchToLive,
    switchToSimulation,
  };
}

export default useBackendConnection;