import React, { useEffect, useRef, useState } from 'react';
import { WAYPOINTS, AFGHANISTAN_CENTER } from '../data/seedData';

const TacticalMap = ({ drones, selectedDrone, onDroneSelect }) => {
  const mapRef = useRef(null);
  const mapInstanceRef = useRef(null);
  const droneMarkersRef = useRef({});
  const [mapsReady, setMapsReady] = useState(false);

  // Wait for Google Maps to be ready
  useEffect(() => {
    const checkGoogleMaps = () => {
      if (window.google && window.google.maps) {
        setMapsReady(true);
      }
    };

    // Check if already loaded
    checkGoogleMaps();

    // Listen for maps ready event
    window.addEventListener('google-maps-ready', checkGoogleMaps);

    // Polling fallback
    const interval = setInterval(() => {
      if (window.google && window.google.maps) {
        setMapsReady(true);
        clearInterval(interval);
      }
    }, 100);

    return () => {
      window.removeEventListener('google-maps-ready', checkGoogleMaps);
      clearInterval(interval);
    };
  }, []);

  // Initialize map
  useEffect(() => {
    if (!mapsReady || !mapRef.current || mapInstanceRef.current) return;

    // DEBUG: Check container dimensions
    console.log('Map container dimensions:', {
      width: mapRef.current.offsetWidth,
      height: mapRef.current.offsetHeight,
      element: mapRef.current
    });

    // Force a minimum height if container has no height
    if (mapRef.current.offsetHeight === 0) {
      console.warn('Map container has no height! Setting minimum height...');
      mapRef.current.style.minHeight = '500px';
    }

    try {
      const map = new window.google.maps.Map(mapRef.current, {
        center: AFGHANISTAN_CENTER,
        zoom: 12,
        mapTypeId: 'satellite',
        disableDefaultUI: false,
        zoomControl: true,
        mapTypeControl: true,
        scaleControl: true,
        streetViewControl: false,
        rotateControl: false,
        fullscreenControl: true,
        styles: [
          {
            featureType: "all",
            elementType: "labels.text.fill",
            stylers: [{ color: "#00ff00" }]
          },
          {
            featureType: "all",
            elementType: "labels.text.stroke",
            stylers: [{ color: "#000000" }, { lightness: 13 }]
          },
          {
            featureType: "water",
            elementType: "geometry",
            stylers: [{ color: "#001a1a" }]
          },
          {
            featureType: "landscape",
            elementType: "geometry",
            stylers: [{ color: "#0a0f0a" }]
          }
        ]
      });

      mapInstanceRef.current = map;
      console.log('Map instance created:', map);

      // Add waypoint markers
      WAYPOINTS.forEach((waypoint, index) => {
        const marker = new window.google.maps.Marker({
          position: { lat: waypoint.lat, lng: waypoint.lng },
          map: map,
          title: waypoint.name,
          icon: {
            path: window.google.maps.SymbolPath.CIRCLE,
            scale: 10,
            fillColor: '#ff6b35',
            fillOpacity: 0.9,
            strokeColor: '#ffffff',
            strokeWeight: 2,
            anchor: new window.google.maps.Point(0, 0)
          },
          zIndex: 1
        });

        // Create info window for waypoint
        const infoWindow = new window.google.maps.InfoWindow({
          content: `
            <div style="
              background: rgba(0, 0, 0, 0.9);
              color: #00ff00;
              padding: 8px;
              border: 1px solid #00ff00;
              font-family: 'JetBrains Mono', monospace;
              font-size: 12px;
              min-width: 150px;
            ">
              <strong>${waypoint.name}</strong><br/>
              <span style="color: #0088ff;">WP-${(index + 1).toString().padStart(2, '0')}</span><br/>
              <span style="color: #888;">LAT: ${waypoint.lat.toFixed(4)}</span><br/>
              <span style="color: #888;">LNG: ${waypoint.lng.toFixed(4)}</span>
            </div>
          `,
          maxWidth: 200
        });

        marker.addListener('click', () => {
          infoWindow.open(map, marker);
        });
      });

      // Draw route path
      const routePath = new window.google.maps.Polyline({
        path: WAYPOINTS.map(wp => ({ lat: wp.lat, lng: wp.lng })),
        geodesic: true,
        strokeColor: '#00ff00',
        strokeOpacity: 0.4,
        strokeWeight: 2,
        zIndex: 0
      });
      routePath.setMap(map);

      console.log('Tactical map initialized successfully');
      
      // Force a resize after a short delay to ensure proper rendering
      setTimeout(() => {
        google.maps.event.trigger(map, 'resize');
        map.setCenter(AFGHANISTAN_CENTER);
        console.log('Map resize triggered');
      }, 100);

    } catch (error) {
      console.error('Error initializing map:', error);
    }
  }, [mapsReady]);

  // Update drone positions
  useEffect(() => {
    console.log('TacticalMap useEffect triggered, drones:', drones?.length);
    if (!mapInstanceRef.current || !window.google || !drones) return;

    // Update existing markers or create new ones
    drones.forEach(drone => {
      console.log('Processing drone:', drone.id, 'lat:', drone.lat, 'lng:', drone.lng);
      // Use direct lat/lng from backend if available, otherwise interpolate from waypoints
      let lat, lng;
      if (drone.lat && drone.lng && drone.lat !== 0) {
        // LIVE mode: use backend coordinates
        lat = drone.lat;
        lng = drone.lng;
      } else {
        // SIMULATION mode: interpolate from waypoints
        const currentWP = WAYPOINTS[drone.currentWaypoint];
        const nextWP = WAYPOINTS[Math.min(drone.currentWaypoint + 1, WAYPOINTS.length - 1)];
        if (currentWP && nextWP) {
          lat = currentWP.lat + (nextWP.lat - currentWP.lat) * drone.progress;
          lng = currentWP.lng + (nextWP.lng - currentWP.lng) * drone.progress;
        } else {
          return; // Skip if no valid position
        }
      }

      // Get waypoints for heading calculation
      const currentWP = WAYPOINTS[drone.currentWaypoint] || WAYPOINTS[0];
      const nextWP = WAYPOINTS[Math.min((drone.currentWaypoint || 0) + 1, WAYPOINTS.length - 1)];
      
      // Calculate heading if geometry library is available
      let heading = 0;
      if (window.google.maps.geometry && window.google.maps.geometry.spherical && currentWP && nextWP) {
        heading = window.google.maps.geometry.spherical.computeHeading(
          new window.google.maps.LatLng(currentWP.lat, currentWP.lng),
          new window.google.maps.LatLng(nextWP.lat, nextWP.lng)
        );
      }

      const droneIcon = {
        path: 'M362.7 19.3L314.3 67.7 444.3 197.7l48.4-48.4c25-25 25-65.5 0-90.5L453.3 19.3c-25-25-65.5-25-90.5 0zm-71 71L58.6 323.5c-10.4 10.4-18 23.3-22.2 37.4L1 481.2C-1.5 489.7 .8 498.8 7 505s15.3 8.5 23.7 6.1l120.3-35.4c14.1-4.2 27-11.8 37.4-22.2L421.7 220.3 291.7 90.3z',
        fillColor: drone.status === 'online' ? '#00ff00' : '#ff0000',
        fillOpacity: 0.9,
        strokeColor: '#ffffff',
        strokeWeight: 1,
        scale: 0.04,
        rotation: heading,
        anchor: new window.google.maps.Point(256, 256)
      };

      // Check if marker already exists
      if (droneMarkersRef.current[drone.id]) {
        // Update existing marker position
        droneMarkersRef.current[drone.id].setPosition({ lat, lng });
        droneMarkersRef.current[drone.id].setIcon(droneIcon);
      } else {
        // Create new marker
        const marker = new window.google.maps.Marker({
          position: { lat, lng },
          map: mapInstanceRef.current,
          title: drone.callsign,
          icon: droneIcon,
          zIndex: 10
        });

        // Create info window for drone
        const infoWindow = new window.google.maps.InfoWindow({
          content: `
            <div style="
              background: rgba(0, 0, 0, 0.95);
              color: #00ff00;
              padding: 10px;
              border: 2px solid ${drone.status === 'online' ? '#00ff00' : '#ff0000'};
              font-family: 'JetBrains Mono', monospace;
              font-size: 11px;
              min-width: 200px;
            ">
              <strong style="font-size: 14px;">${drone.callsign}</strong><br/>
              <div style="margin-top: 5px;">
                <span style="color: ${drone.status === 'online' ? '#00ff00' : '#ff0000'};">
                  STATUS: ${drone.status.toUpperCase()}
                </span><br/>
                <span style="color: #0088ff;">ALT: ${drone.altitude?.toFixed(0) || 0}m</span><br/>
                <span style="color: #0088ff;">SPD: ${drone.speed?.toFixed(0) || 0} km/h</span><br/>
                <span style="color: #ffaa00;">BAT: ${drone.battery?.toFixed(0) || 0}%</span><br/>
                <span style="color: #888;">LAT: ${lat.toFixed(4)}</span><br/>
                <span style="color: #888;">LNG: ${lng.toFixed(4)}</span>
              </div>
            </div>
          `,
          maxWidth: 250
        });

        marker.addListener('click', () => {
          infoWindow.open(mapInstanceRef.current, marker);
          if (onDroneSelect) {
            onDroneSelect(drone);
          }
        });

        droneMarkersRef.current[drone.id] = marker;
      }
    });

    // Remove markers for drones that no longer exist
    Object.keys(droneMarkersRef.current).forEach(droneId => {
      if (!drones.find(d => d.id === droneId)) {
        droneMarkersRef.current[droneId].setMap(null);
        delete droneMarkersRef.current[droneId];
      }
    });

  }, [drones, selectedDrone, onDroneSelect]);

  // Loading state
  if (!mapsReady) {
    return (
      <div style={{ 
        width: '100%', 
        height: '100%', 
        background: '#0a0a0a',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        color: '#00ff00'
      }}>
        <div style={{ textAlign: 'center' }}>
          <div style={{ fontSize: '20px', marginBottom: '10px' }}>LOADING TACTICAL MAP...</div>
          <div style={{ fontSize: '14px', color: '#666' }}>Establishing satellite connection</div>
        </div>
      </div>
    );
  }

  return (
    <div style={{ 
      position: 'relative', 
      width: '100%', 
      height: '100%',
      background: '#1a1a1a'
    }}>
      <div 
        ref={mapRef} 
        style={{ 
          width: '100%', 
          height: '100%',
          minHeight: '500px'
        }} 
      />
      
      {/* Map overlay with status */}
      <div style={{
        position: 'absolute',
        top: '16px',
        left: '16px',
        background: 'rgba(0, 0, 0, 0.8)',
        color: '#00ff00',
        padding: '12px',
        borderRadius: '4px',
        border: '1px solid #00ff00',
        fontSize: '12px',
        fontFamily: 'JetBrains Mono, monospace'
      }}>
        <div>TACTICAL OVERVIEW</div>
        <div style={{ color: '#888' }}>Region: Afghanistan</div>
        <div style={{ color: '#888' }}>Grid: Active</div>
      </div>
    </div>
  );
};

export default TacticalMap;