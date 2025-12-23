-- ============================================================================
-- Drone Convoy Tracking System - ScyllaDB Schema
-- ============================================================================
-- This schema stores drone telemetry, waypoint events, tracking data,
-- and OpenCV detection results for real-time convoy management.
-- ============================================================================

-- Create keyspace with NetworkTopologyStrategy for production
-- Using SimpleStrategy here for local development with replication factor 3
CREATE KEYSPACE IF NOT EXISTS drone_convoy
WITH replication = {
    'class': 'SimpleStrategy',
    'replication_factor': 3
}
AND durable_writes = true;

USE drone_convoy;

-- ============================================================================
-- DRONE TELEMETRY TABLE
-- Time-series data for real-time drone status and position
-- Partitioned by drone_id with clustering by timestamp for efficient range queries
-- ============================================================================
CREATE TABLE IF NOT EXISTS drone_telemetry (
    drone_id        TEXT,
    timestamp       TIMESTAMP,
    -- Position data
    latitude        DOUBLE,
    longitude       DOUBLE,
    altitude        DOUBLE,
    heading         DOUBLE,
    speed           DOUBLE,
    -- System status
    battery_level   INT,
    fuel_level      INT,
    system_health   INT,
    -- Status flags
    status          TEXT,      -- STANDBY, MOVING, ENGAGED, RTB, OFFLINE
    armed           BOOLEAN,
    -- Sensor data
    temperature     DOUBLE,
    signal_strength INT,
    -- Metadata
    mission_id      UUID,
    PRIMARY KEY ((drone_id), timestamp)
) WITH CLUSTERING ORDER BY (timestamp DESC)
   AND default_time_to_live = 604800  -- 7 days TTL
   AND compaction = {
       'class': 'TimeWindowCompactionStrategy',
       'compaction_window_size': 1,
       'compaction_window_unit': 'DAYS'
   }
   AND compression = {
       'sstable_compression': 'LZ4Compressor'
   };

-- Index for querying by status across all drones
CREATE INDEX IF NOT EXISTS idx_telemetry_status ON drone_telemetry (status);

-- ============================================================================
-- WAYPOINT EVENTS TABLE
-- Records when drones pass through waypoints
-- ============================================================================
CREATE TABLE IF NOT EXISTS waypoint_events (
    mission_id      UUID,
    event_time      TIMESTAMP,
    drone_id        TEXT,
    waypoint_id     TEXT,
    waypoint_name   TEXT,
    latitude        DOUBLE,
    longitude       DOUBLE,
    -- Event details
    event_type      TEXT,      -- ARRIVAL, DEPARTURE, FLYOVER
    speed_at_event  DOUBLE,
    altitude_at_event DOUBLE,
    heading         DOUBLE,
    -- Timing
    eta_next        TIMESTAMP,
    time_on_station DURATION,
    PRIMARY KEY ((mission_id), event_time, drone_id)
) WITH CLUSTERING ORDER BY (event_time DESC, drone_id ASC)
   AND default_time_to_live = 2592000;  -- 30 days TTL

-- ============================================================================
-- OPENCV TRACKING TABLE
-- Stores computer vision detection and tracking results
-- ============================================================================
CREATE TABLE IF NOT EXISTS cv_tracking (
    drone_id        TEXT,
    frame_timestamp TIMESTAMP,
    -- Detection box coordinates (pixels)
    bbox_x          INT,
    bbox_y          INT,
    bbox_width      INT,
    bbox_height     INT,
    -- Tracking data
    tracking_id     INT,
    confidence      DOUBLE,
    -- Halo detection
    halo_detected   BOOLEAN,
    halo_center_x   INT,
    halo_center_y   INT,
    halo_radius     INT,
    halo_color_r    INT,
    halo_color_g    INT,
    halo_color_b    INT,
    -- Geo-projection (estimated from visual)
    est_latitude    DOUBLE,
    est_longitude   DOUBLE,
    -- Kalman filter state
    kalman_state    BLOB,
    PRIMARY KEY ((drone_id), frame_timestamp)
) WITH CLUSTERING ORDER BY (frame_timestamp DESC)
   AND default_time_to_live = 86400  -- 24 hours TTL for CV data
   AND compaction = {
       'class': 'TimeWindowCompactionStrategy',
       'compaction_window_size': 1,
       'compaction_window_unit': 'HOURS'
   };

-- ============================================================================
-- MISSION TABLE
-- Stores mission configurations and status
-- ============================================================================
CREATE TABLE IF NOT EXISTS missions (
    mission_id      UUID,
    created_at      TIMESTAMP,
    -- Mission details
    name            TEXT,
    description     TEXT,
    status          TEXT,      -- PLANNING, ACTIVE, PAUSED, COMPLETED, ABORTED
    -- Timing
    start_time      TIMESTAMP,
    end_time        TIMESTAMP,
    -- Configuration
    waypoints       LIST<FROZEN<TUPLE<TEXT, DOUBLE, DOUBLE>>>,  -- (name, lat, lng)
    assigned_drones LIST<TEXT>,
    -- Metadata
    created_by      TEXT,
    updated_at      TIMESTAMP,
    PRIMARY KEY (mission_id)
);

CREATE INDEX IF NOT EXISTS idx_missions_status ON missions (status);

-- ============================================================================
-- DRONE REGISTRY TABLE
-- Master list of all drones in the system
-- ============================================================================
CREATE TABLE IF NOT EXISTS drone_registry (
    drone_id        TEXT PRIMARY KEY,
    -- Identification
    callsign        TEXT,
    drone_type      TEXT,      -- REAPER, PREDATOR, GLOBAL_HAWK, etc.
    serial_number   TEXT,
    -- Capabilities
    max_altitude    DOUBLE,
    max_speed       DOUBLE,
    max_range       DOUBLE,
    payload_capacity DOUBLE,
    -- Current assignment
    current_mission UUID,
    home_base       TEXT,
    -- Status
    operational     BOOLEAN,
    last_maintenance TIMESTAMP,
    -- Metadata
    registered_at   TIMESTAMP,
    updated_at      TIMESTAMP
);

-- ============================================================================
-- ALERTS TABLE
-- System alerts and warnings
-- ============================================================================
CREATE TABLE IF NOT EXISTS alerts (
    alert_id        UUID,
    created_at      TIMESTAMP,
    -- Alert details
    severity        TEXT,      -- INFO, WARNING, CRITICAL, EMERGENCY
    alert_type      TEXT,      -- BATTERY_LOW, SIGNAL_LOST, WAYPOINT_DEVIATION, etc.
    message         TEXT,
    -- Source
    drone_id        TEXT,
    mission_id      UUID,
    -- Resolution
    acknowledged    BOOLEAN,
    acknowledged_by TEXT,
    acknowledged_at TIMESTAMP,
    resolved        BOOLEAN,
    resolved_at     TIMESTAMP,
    PRIMARY KEY ((drone_id), created_at, alert_id)
) WITH CLUSTERING ORDER BY (created_at DESC, alert_id ASC)
   AND default_time_to_live = 2592000;  -- 30 days TTL

-- ============================================================================
-- P2P NETWORK METRICS TABLE
-- Stores mesh network health and communication stats
-- ============================================================================
CREATE TABLE IF NOT EXISTS p2p_metrics (
    node_id         TEXT,
    timestamp       TIMESTAMP,
    -- Network stats
    peer_count      INT,
    connected_peers LIST<TEXT>,
    messages_sent   BIGINT,
    messages_received BIGINT,
    bytes_sent      BIGINT,
    bytes_received  BIGINT,
    -- Latency
    avg_latency_ms  DOUBLE,
    max_latency_ms  DOUBLE,
    -- Health
    packet_loss_pct DOUBLE,
    PRIMARY KEY ((node_id), timestamp)
) WITH CLUSTERING ORDER BY (timestamp DESC)
   AND default_time_to_live = 86400;  -- 24 hours TTL

-- ============================================================================
-- MATERIALIZED VIEWS FOR COMMON QUERIES
-- ============================================================================

-- View: Latest position of all drones (for map display)
CREATE MATERIALIZED VIEW IF NOT EXISTS latest_drone_positions AS
    SELECT drone_id, timestamp, latitude, longitude, altitude, 
           heading, speed, status, battery_level, system_health
    FROM drone_telemetry
    WHERE drone_id IS NOT NULL 
      AND timestamp IS NOT NULL 
      AND latitude IS NOT NULL
    PRIMARY KEY (status, drone_id, timestamp)
    WITH CLUSTERING ORDER BY (drone_id ASC, timestamp DESC);

-- ============================================================================
-- USER-DEFINED TYPES
-- ============================================================================

-- GeoPoint type for coordinates
CREATE TYPE IF NOT EXISTS geo_point (
    latitude  DOUBLE,
    longitude DOUBLE,
    altitude  DOUBLE
);

-- BoundingBox type for CV detections
CREATE TYPE IF NOT EXISTS bounding_box (
    x      INT,
    y      INT,
    width  INT,
    height INT
);

-- ============================================================================
-- SAMPLE DATA FOR TESTING
-- ============================================================================

-- Insert sample mission
INSERT INTO missions (mission_id, created_at, name, description, status, waypoints, assigned_drones)
VALUES (
    uuid(),
    toTimestamp(now()),
    'Operation Desert Watch',
    'Convoy escort mission across 12 strategic waypoints in Afghanistan',
    'ACTIVE',
    [
        ('Base Alpha', 34.5553, 69.2075),
        ('Checkpoint Bravo', 34.6234, 69.1123),
        ('Outpost Charlie', 34.7012, 69.0456),
        ('Firebase Delta', 34.7891, 68.9234),
        ('Sector Echo', 34.8567, 68.8012),
        ('Point Foxtrot', 34.9234, 68.6789),
        ('Zone Golf', 34.9901, 68.5567),
        ('Camp Hotel', 35.0567, 68.4234),
        ('Station India', 35.1234, 68.3012),
        ('Forward Juliet', 35.1901, 68.1789),
        ('Base Kilo', 35.2567, 68.0567),
        ('Terminal Lima', 35.3234, 67.9234)
    ],
    ['REAPER-01', 'REAPER-02', 'REAPER-03', 'REAPER-04', 'REAPER-05', 'REAPER-06',
     'REAPER-07', 'REAPER-08', 'REAPER-09', 'REAPER-10', 'REAPER-11', 'REAPER-12']
);

-- Insert sample drones
INSERT INTO drone_registry (drone_id, callsign, drone_type, max_altitude, max_speed, operational, registered_at)
VALUES ('REAPER-01', 'Alpha Lead', 'MQ-9 REAPER', 15000.0, 480.0, true, toTimestamp(now()));

INSERT INTO drone_registry (drone_id, callsign, drone_type, max_altitude, max_speed, operational, registered_at)
VALUES ('REAPER-02', 'Alpha Two', 'MQ-9 REAPER', 15000.0, 480.0, true, toTimestamp(now()));

INSERT INTO drone_registry (drone_id, callsign, drone_type, max_altitude, max_speed, operational, registered_at)
VALUES ('REAPER-03', 'Alpha Three', 'MQ-9 REAPER', 15000.0, 480.0, true, toTimestamp(now()));
