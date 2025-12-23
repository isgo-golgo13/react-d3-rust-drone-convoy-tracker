import React, { useState } from 'react';
import TacticalMap from './components/TacticalMap';
import DroneControlPanel from './components/DroneControlPanel';
import ConvoyProgress from './components/ConvoyProgress';
import ConnectionStatus from './components/ConnectionStatus';
import { useBackendConnection } from './hooks/useBackendConnection';
import { WAYPOINTS } from './data/seedData';
import './App.css';

function App() {
  // Use backend connection hook (replaces local state + simulation)
  const {
    drones,
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
    error,
    switchToLive,
    switchToSimulation,
  } = useBackendConnection();

  // Selected drone (local UI state)
  const [selectedDrone, setSelectedDrone] = useState(null);

  // Handlers for simulation controls
  const handleStartSimulation = () => startSimulation();
  const handlePauseSimulation = () => stopSimulation();
  const handleResetSimulation = () => {
    resetSimulation();
    setSelectedDrone(null);
  };

  return (
    <div className="app-container">
      {/* Header */}
      <header className="app-header">
        <h1 className="app-title">
          <span className="glitch" data-text="DRONE CONVOY SORTIE">DRONE CONVOY SORTIE</span>
        </h1>
        
        <div style={{ display: 'flex', alignItems: 'center', gap: '20px' }}>
          {/* Connection Status */}
          <ConnectionStatus
            mode={mode}
            isConnected={isConnected}
            error={error}
            onSwitchToLive={switchToLive}
            onSwitchToSimulation={switchToSimulation}
          />
          
          {/* Original status indicator */}
          <div className="header-info">
            <span className={`status-indicator ${isConnected ? 'online' : 'offline'}`}></span>
            <span>TACTICAL COMMAND {isConnected ? 'LINKED' : 'LOCAL'}</span>
          </div>
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
