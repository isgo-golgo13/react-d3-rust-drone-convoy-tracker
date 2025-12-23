import React, { useState, useEffect, useRef } from 'react';
import TacticalMap from './components/TacticalMap';
import DroneControlPanel from './components/DroneControlPanel';
import ConvoyProgress from './components/ConvoyProgress';
import { WAYPOINTS } from './data/seedData';
import './App.css';

// Initialize 12 military attack drones
const initializeDrones = () => {
  const drones = [];
  for (let i = 1; i <= 12; i++) {
    drones.push({
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
      lat: 0,
      lng: 0
    });
  }
  return drones;
};

function App() {
  const [drones, setDrones] = useState(initializeDrones());
  const [selectedDrone, setSelectedDrone] = useState(null);
  const [isSimulating, setIsSimulating] = useState(false);
  const [simulationSpeed, setSimulationSpeed] = useState(1);
  const simulationIntervalRef = useRef(null);

  // Simulation logic
  useEffect(() => {
    if (!isSimulating) {
      if (simulationIntervalRef.current) {
        clearInterval(simulationIntervalRef.current);
      }
      return;
    }

    simulationIntervalRef.current = setInterval(() => {
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
            systemHealth: Math.max(70, drone.systemHealth - Math.random() * 0.1)
          };
        })
      );
    }, 100);

    return () => {
      if (simulationIntervalRef.current) {
        clearInterval(simulationIntervalRef.current);
      }
    };
  }, [isSimulating, simulationSpeed]);

  const handleStartSimulation = () => setIsSimulating(true);
  const handlePauseSimulation = () => setIsSimulating(false);
  const handleResetSimulation = () => {
    setIsSimulating(false);
    setDrones(initializeDrones());
    setSelectedDrone(null);
  };

  return (
    <div className="app-container">
      {/* Header */}
      <header className="app-header">
        <h1 className="app-title">
          <span className="glitch" data-text="DRONE CONVOY SORTIE">DRONE CONVOY SORTIE</span>
        </h1>
        <div className="header-info">
          <span className="status-indicator online"></span>
          <span>TACTICAL COMMAND ACTIVE</span>
        </div>
      </header>

      {/* Main Content */}
      <div className="main-content">
        {/* Left Panel - Map */}
        <div className="map-panel">
          <TacticalMap 
            drones={drones} 
            selectedDrone={selectedDrone}
            onDroneSelect={setSelectedDrone}
          />
        </div>

        {/* Right Panel - Controls */}
        <div className="control-panel">
          <DroneControlPanel
            drones={drones}
            selectedDrone={selectedDrone}
            onDroneSelect={setSelectedDrone}
            isSimulating={isSimulating}
            simulationSpeed={simulationSpeed}
            onStartSimulation={handleStartSimulation}
            onPauseSimulation={handlePauseSimulation}
            onResetSimulation={handleResetSimulation}
            onSpeedChange={setSimulationSpeed}
          />
        </div>
      </div>

      {/* Bottom Panel - Progress Visualization */}
      <div className="progress-panel">
        <ConvoyProgress drones={drones} waypoints={WAYPOINTS} />
      </div>
    </div>
  );
}

export default App;