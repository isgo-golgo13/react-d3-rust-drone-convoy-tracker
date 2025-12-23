/**
 * ConnectionStatus Component
 * 
 * Shows backend connection status with military-style UI.
 * Matches existing App.css styling.
 */

import React from 'react';
import { ConnectionMode } from '../hooks/useBackendConnection';

export function ConnectionStatus({ 
  mode, 
  isConnected, 
  error, 
  onSwitchToLive, 
  onSwitchToSimulation 
}) {
  const getStatusConfig = () => {
    switch (mode) {
      case ConnectionMode.LIVE:
        return { 
          icon: '🟢', 
          text: 'LIVE', 
          color: '#00ff00',
          bgColor: 'rgba(0, 255, 0, 0.2)',
          borderColor: '#00ff00'
        };
      case ConnectionMode.SIMULATION:
        return { 
          icon: '🟡', 
          text: 'SIM', 
          color: '#ffaa00',
          bgColor: 'rgba(255, 170, 0, 0.2)',
          borderColor: '#ffaa00'
        };
      case ConnectionMode.CONNECTING:
        return { 
          icon: '🔵', 
          text: 'CONNECTING', 
          color: '#00aaff',
          bgColor: 'rgba(0, 170, 255, 0.2)',
          borderColor: '#00aaff'
        };
      case ConnectionMode.ERROR:
        return { 
          icon: '🔴', 
          text: 'ERROR', 
          color: '#ff0000',
          bgColor: 'rgba(255, 0, 0, 0.2)',
          borderColor: '#ff0000'
        };
      default:
        return { 
          icon: '⚪', 
          text: 'UNKNOWN', 
          color: '#888888',
          bgColor: 'rgba(136, 136, 136, 0.2)',
          borderColor: '#888888'
        };
    }
  };

  const config = getStatusConfig();

  const containerStyle = {
    display: 'flex',
    alignItems: 'center',
    gap: '12px',
    padding: '8px 16px',
    background: '#1a1a1a',
    borderRadius: '4px',
    border: '1px solid #333',
    fontFamily: "'JetBrains Mono', monospace",
    fontSize: '11px',
  };

  const statusBadgeStyle = {
    display: 'flex',
    alignItems: 'center',
    gap: '6px',
    padding: '4px 10px',
    background: config.bgColor,
    border: `1px solid ${config.borderColor}`,
    borderRadius: '4px',
    color: config.color,
    fontWeight: 'bold',
    textTransform: 'uppercase',
    letterSpacing: '1px',
  };

  const wsIndicatorStyle = {
    display: 'flex',
    alignItems: 'center',
    gap: '6px',
    color: '#888',
  };

  const wsDotStyle = {
    width: '8px',
    height: '8px',
    borderRadius: '50%',
    background: isConnected ? '#00ff00' : '#ff0000',
    boxShadow: isConnected ? '0 0 8px #00ff00' : '0 0 8px #ff0000',
  };

  const buttonStyle = (isActive, color) => ({
    padding: '4px 10px',
    background: isActive ? color : '#0a0a0a',
    border: `1px solid ${color}`,
    color: isActive ? '#0a0a0a' : color,
    borderRadius: '4px',
    cursor: isActive ? 'default' : 'pointer',
    fontFamily: "'JetBrains Mono', monospace",
    fontSize: '10px',
    textTransform: 'uppercase',
    transition: 'all 0.2s ease',
    opacity: isActive ? 1 : 0.7,
  });

  return (
    <div style={containerStyle}>
      {/* Status Badge */}
      <div style={statusBadgeStyle}>
        <span>{config.icon}</span>
        <span>{config.text}</span>
      </div>

      {/* WebSocket Status */}
      <div style={wsIndicatorStyle}>
        <span style={wsDotStyle}></span>
        <span>WS: {isConnected ? 'ON' : 'OFF'}</span>
      </div>

      {/* Mode Toggle */}
      <div style={{ display: 'flex', gap: '4px' }}>
        <button
          onClick={onSwitchToLive}
          disabled={mode === ConnectionMode.LIVE || mode === ConnectionMode.CONNECTING}
          style={buttonStyle(mode === ConnectionMode.LIVE, '#00ff00')}
          title="Connect to backend"
        >
          LIVE
        </button>
        <button
          onClick={onSwitchToSimulation}
          disabled={mode === ConnectionMode.SIMULATION}
          style={buttonStyle(mode === ConnectionMode.SIMULATION, '#ffaa00')}
          title="Use local simulation"
        >
          SIM
        </button>
      </div>

      {/* Error indicator */}
      {error && (
        <span style={{ color: '#ff6666', fontSize: '10px' }} title={error}>
          ⚠️
        </span>
      )}
    </div>
  );
}

export default ConnectionStatus;
