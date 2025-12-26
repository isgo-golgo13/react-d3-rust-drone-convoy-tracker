import { WAYPOINTS } from '../data/seedData.js';

/**
 * Calculate the current lat/lng position of a drone based on its waypoint progress
 * @param {Object} drone - The drone object
 * @returns {Object} - {lat, lng} coordinates
 */
export const calculateDronePosition = (drone) => {
  // Use direct lat/lng from backend if available
  if (drone.lat && drone.lng && drone.lat !== 0) {
    return { lat: drone.lat, lng: drone.lng };
  }
  
  // Fallback: interpolate from waypoints (simulation mode)
  if (!drone || drone.currentWaypoint >= WAYPOINTS.length - 1) {
    return WAYPOINTS[WAYPOINTS.length - 1] || { lat: 0, lng: 0 };
  }

  const currentWP = WAYPOINTS[drone.currentWaypoint];
  const nextWP = WAYPOINTS[drone.currentWaypoint + 1];

  if (!currentWP || !nextWP) {
    return currentWP || { lat: 0, lng: 0 };
  }

  const lat = currentWP.lat + (nextWP.lat - currentWP.lat) * drone.progress;
  const lng = currentWP.lng + (nextWP.lng - currentWP.lng) * drone.progress;

  return { lat, lng };
};

/**
 * Calculate distance between two coordinates using Haversine formula
 * @param {Object} pos1 - {lat, lng}
 * @param {Object} pos2 - {lat, lng}
 * @returns {number} - Distance in kilometers
 */
export const calculateDistance = (pos1, pos2) => {
  const R = 6371; // Earth's radius in km
  const dLat = toRad(pos2.lat - pos1.lat);
  const dLng = toRad(pos2.lng - pos1.lng);
  
  const a = Math.sin(dLat/2) * Math.sin(dLat/2) +
           Math.cos(toRad(pos1.lat)) * Math.cos(toRad(pos2.lat)) *
           Math.sin(dLng/2) * Math.sin(dLng/2);
  
  const c = 2 * Math.atan2(Math.sqrt(a), Math.sqrt(1-a));
  return R * c;
};

/**
 * Convert degrees to radians
 * @param {number} deg
 * @returns {number}
 */
const toRad = (deg) => deg * (Math.PI / 180);

/**
 * Calculate bearing between two coordinates
 * @param {Object} pos1 - {lat, lng}
 * @param {Object} pos2 - {lat, lng}
 * @returns {number} - Bearing in degrees
 */
export const calculateBearing = (pos1, pos2) => {
  const dLng = toRad(pos2.lng - pos1.lng);
  const lat1 = toRad(pos1.lat);
  const lat2 = toRad(pos2.lat);
  
  const y = Math.sin(dLng) * Math.cos(lat2);
  const x = Math.cos(lat1) * Math.sin(lat2) - Math.sin(lat1) * Math.cos(lat2) * Math.cos(dLng);
  
  const bearing = Math.atan2(y, x);
  return (bearing * 180 / Math.PI + 360) % 360;
};

/**
 * Get status color based on drone status
 * @param {string} status
 * @returns {string} - CSS class name
 */
export const getStatusColor = (status) => {
  const statusMap = {
    'online': 'status-online',
    'offline': 'status-offline',
    'warning': 'status-warning',
    'critical': 'status-offline',
    'maintenance': 'status-warning'
  };
  return statusMap[status] || 'status-offline';
};

/**
 * Get status text color for UI
 * @param {string} status
 * @returns {string} - Tailwind color class
 */
export const getStatusTextColor = (status) => {
  const statusMap = {
    'online': 'text-military-success',
    'offline': 'text-military-danger',
    'warning': 'text-military-warning',
    'critical': 'text-red-400',
    'maintenance': 'text-yellow-400'
  };
  return statusMap[status] || 'text-gray-400';
};

/**
 * Format coordinates for display
 * @param {Object} position - {lat, lng}
 * @returns {string} - Formatted coordinate string
 */
export const formatCoordinates = (position) => {
  if (!position || !position.lat || !position.lng) {
    return 'N/A';
  }
  
  const lat = Math.abs(position.lat).toFixed(6);
  const lng = Math.abs(position.lng).toFixed(6);
  const latDir = position.lat >= 0 ? 'N' : 'S';
  const lngDir = position.lng >= 0 ? 'E' : 'W';
  
  return `${lat}°${latDir} ${lng}°${lngDir}`;
};

/**
 * Get system health status
 * @param {number} health - Health percentage
 * @returns {Object} - {status, color, text}
 */
export const getSystemHealthStatus = (health) => {
  if (health >= 90) {
    return { status: 'excellent', color: 'text-green-400', text: 'EXCELLENT' };
  } else if (health >= 75) {
    return { status: 'good', color: 'text-blue-400', text: 'GOOD' };
  } else if (health >= 50) {
    return { status: 'fair', color: 'text-yellow-400', text: 'FAIR' };
  } else if (health >= 25) {
    return { status: 'poor', color: 'text-orange-400', text: 'POOR' };
  } else {
    return { status: 'critical', color: 'text-red-400', text: 'CRITICAL' };
  }
};

/**
 * Calculate ETA to next waypoint
 * @param {Object} drone
 * @returns {number} - ETA in minutes
 */
export const calculateETA = (drone) => {
  if (!drone || drone.speed <= 0) return null;
  
  const currentPos = calculateDronePosition(drone);
  const nextWaypoint = WAYPOINTS[drone.currentWaypoint + 1];
  
  if (!nextWaypoint) return null;
  
  const remainingDistance = calculateDistance(currentPos, nextWaypoint);
  const speedKmh = drone.speed * 1.852; // Convert knots to km/h
  
  return (remainingDistance / speedKmh) * 60; // Convert to minutes
};

/**
 * Generate mission time string
 * @param {Date} startTime
 * @returns {string}
 */
export const formatMissionTime = (startTime) => {
  const now = new Date();
  const diff = now - startTime;
  const minutes = Math.floor(diff / 60000);
  const hours = Math.floor(minutes / 60);
  const remainingMinutes = minutes % 60;
  
  if (hours > 0) {
    return `${hours.toString().padStart(2, '0')}:${remainingMinutes.toString().padStart(2, '0')}`;
  }
  return `${remainingMinutes}m`;
};

/**
 * Check if drone needs attention based on multiple factors
 * @param {Object} drone
 * @returns {Object} - {needsAttention, reasons}
 */
export const checkDroneHealth = (drone) => {
  const reasons = [];
  
  if (drone.battery < 30) reasons.push('Low battery');
  if (drone.fuel < 25) reasons.push('Low fuel');
  if (drone.systemHealth < 75) reasons.push('System degraded');
  if (drone.status === 'offline') reasons.push('Communication lost');
  if (drone.status === 'warning') reasons.push('Warning condition');
  
  const timeSinceUpdate = (new Date() - new Date(drone.lastUpdate)) / 1000;
  if (timeSinceUpdate > 120) reasons.push('Stale data');
  
  return {
    needsAttention: reasons.length > 0,
    reasons
  };
};