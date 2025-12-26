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
    // Position for map
    lat: backendDrone.position?.latitude ?? backendDrone.lat ?? 0,
    lng: backendDrone.position?.longitude ?? backendDrone.lng ?? 0,
  };
}

export function useBackendConnection() {
  // Connection state
  const [mode, setMode] = useState(ConnectionMode.CONNECTING);
  const [isConnected, setIsConnected] = useState(false);
  const [error, setError] = useState(null);

  // Data state - matches existing App.jsx structure
  const [drones, setDrones] = useState([]);
  const [isSimulating, setIsSimulating] = useState(false);
  const [simulationSpeed, setSimulationSpeed] = useState(1);

  // Refs
  const wsRef = useRef(null);
  const reconnectAttempts = useRef(0);
  const reconnectTimeout = useRef(null);
  const simulationInterval = useRef(null);

  /**
   * Initialize drones for simulation mode (matches App.jsx initializeDrones)
   */
  const initializeSimulationDrones = useCallback(() => {
    // Use INITIAL_DRONES from seedData if available, otherwise generate
    if (INITIAL_DRONES && INITIAL_DRONES.length > 0) {
      return INITIAL_DRONES.map(drone => ({
        ...drone,
        lastUpdate: new Date(),
      }));
    }

    // Fallback: generate 12 drones
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
      const data = JSON.parse(event.data);
      
      // Handle different event types from Rust backend
      if (data.event_type === 'DronePositionUpdated' || data.type === 'drone_update') {
        const payload = data.payload || data;
        const droneId = payload.drone_id || payload.droneId || payload.id;

        setDrones(prev => prev.map(drone => {
          if (drone.id !== droneId) return drone;

          return {
            ...drone,
            lat: payload.position?.latitude ?? payload.lat ?? drone.lat,
            lng: payload.position?.longitude ?? payload.lng ?? drone.lng,
            altitude: payload.position?.altitude ?? payload.altitude ?? drone.altitude,
            battery: payload.telemetry?.battery_level ?? payload.battery ?? drone.battery,
            fuel: payload.telemetry?.fuel_level ?? payload.fuel ?? drone.fuel,
            speed: payload.telemetry?.speed ?? payload.speed ?? drone.speed,
            systemHealth: payload.telemetry?.system_health ?? payload.systemHealth ?? drone.systemHealth,
            currentWaypoint: payload.waypoint_index ?? payload.currentWaypoint ?? drone.currentWaypoint,
            progress: payload.waypoint_progress ?? payload.progress ?? drone.progress,
            status: payload.status?.toLowerCase() ?? drone.status,
            lastUpdate: new Date(),
          };
        }));
      } else if (data.event_type === 'WaypointReached') {
        console.log(`🎯 Drone ${data.payload?.drone_id} reached waypoint`);
      } else if (data.event_type === 'AllDrones' || data.type === 'sync') {
        // Full state sync
        if (data.payload?.drones) {
          setDrones(data.payload.drones.map(transformBackendDrone));
        }
      }
    } catch (err) {
      console.warn('Failed to parse WebSocket message:', err);
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

        // Attempt reconnection if in LIVE mode
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
   * Run simulation update (matches App.jsx simulation logic)
   */
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

        // Update drone stats
        const batteryDrain = 0.1 * simulationSpeed;
        const fuelDrain = 0.15 * simulationSpeed;

        return {
          ...drone,
          progress: newProgress,
          currentWaypoint: newWaypoint,
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
   * Start simulation
   */
  const startSimulation = useCallback(() => {
    setIsSimulating(true);
  }, []);

  /**
   * Stop simulation
   */
  const stopSimulation = useCallback(() => {
    setIsSimulating(false);
  }, []);

  /**
   * Toggle simulation
   */
  const toggleSimulation = useCallback(() => {
    setIsSimulating(prev => !prev);
  }, []);

  /**
   * Reset to initial state
   */
  const resetSimulation = useCallback(() => {
    setIsSimulating(false);
    setDrones(initializeSimulationDrones());
  }, [initializeSimulationDrones]);

  /**
   * Switch to live mode
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

  /**
   * Switch to simulation mode
   */
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
          setDrones(initializeSimulationDrones());
        }
        setMode(ConnectionMode.LIVE);
        connectWebSocket();
      } else {
        console.log('⚠️ Backend unavailable, using SIMULATION mode');
        setDrones(initializeSimulationDrones());
        setMode(ConnectionMode.SIMULATION);
        setIsSimulating(true); // Auto-start simulation
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
    // Data - same shape as original App.jsx
    drones,
    waypoints: WAYPOINTS,
    
    // Simulation controls - same API as original
    isSimulating,
    simulationSpeed,
    startSimulation,
    stopSimulation,
    toggleSimulation,
    resetSimulation,
    setSimulationSpeed,
    
    // Connection status (new)
    mode,
    isConnected,
    isLive: mode === ConnectionMode.LIVE,
    isSimulationMode: mode === ConnectionMode.SIMULATION,
    error,
    
    // Mode switching (new)
    switchToLive,
    switchToSimulation,
  };
}

export default useBackendConnection;
