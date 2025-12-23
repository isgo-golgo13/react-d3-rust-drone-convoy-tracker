//! Geographic types and calculations for drone positioning

use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

/// Earth's radius in kilometers
const EARTH_RADIUS_KM: f64 = 6371.0;

/// Geographic position with latitude, longitude, and altitude
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GeoPosition {
    /// Latitude in degrees (-90 to 90)
    pub latitude: f64,
    /// Longitude in degrees (-180 to 180)
    pub longitude: f64,
    /// Altitude in meters above sea level
    pub altitude: f64,
}

impl Default for GeoPosition {
    fn default() -> Self {
        Self {
            latitude: 0.0,
            longitude: 0.0,
            altitude: 0.0,
        }
    }
}

impl GeoPosition {
    /// Create a new geographic position
    pub fn new(latitude: f64, longitude: f64, altitude: f64) -> Self {
        Self {
            latitude,
            longitude,
            altitude,
        }
    }

    /// Create position from degrees
    pub fn from_degrees(lat_deg: f64, lng_deg: f64) -> Self {
        Self::new(lat_deg, lng_deg, 0.0)
    }

    /// Check if this position is valid
    pub fn is_valid(&self) -> bool {
        self.latitude >= -90.0
            && self.latitude <= 90.0
            && self.longitude >= -180.0
            && self.longitude <= 180.0
    }

    /// Calculate distance to another position using Haversine formula
    /// Returns distance in kilometers
    pub fn distance_to(&self, other: &GeoPosition) -> f64 {
        let lat1 = self.latitude.to_radians();
        let lat2 = other.latitude.to_radians();
        let delta_lat = (other.latitude - self.latitude).to_radians();
        let delta_lng = (other.longitude - self.longitude).to_radians();

        let a = (delta_lat / 2.0).sin().powi(2)
            + lat1.cos() * lat2.cos() * (delta_lng / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().asin();

        EARTH_RADIUS_KM * c
    }

    /// Calculate bearing to another position
    /// Returns bearing in degrees (0-360)
    pub fn bearing_to(&self, other: &GeoPosition) -> f64 {
        let lat1 = self.latitude.to_radians();
        let lat2 = other.latitude.to_radians();
        let delta_lng = (other.longitude - self.longitude).to_radians();

        let y = delta_lng.sin() * lat2.cos();
        let x = lat1.cos() * lat2.sin() - lat1.sin() * lat2.cos() * delta_lng.cos();

        let bearing = y.atan2(x).to_degrees();
        (bearing + 360.0) % 360.0
    }

    /// Calculate a new position given distance and bearing
    /// Distance in kilometers, bearing in degrees
    pub fn destination(&self, distance_km: f64, bearing_deg: f64) -> GeoPosition {
        let lat1 = self.latitude.to_radians();
        let lng1 = self.longitude.to_radians();
        let bearing = bearing_deg.to_radians();
        let angular_distance = distance_km / EARTH_RADIUS_KM;

        let lat2 = (lat1.sin() * angular_distance.cos()
            + lat1.cos() * angular_distance.sin() * bearing.cos())
        .asin();

        let lng2 = lng1
            + (bearing.sin() * angular_distance.sin() * lat1.cos())
                .atan2(angular_distance.cos() - lat1.sin() * lat2.sin());

        GeoPosition::new(lat2.to_degrees(), lng2.to_degrees(), self.altitude)
    }

    /// Interpolate between two positions
    /// fraction: 0.0 = self, 1.0 = other
    pub fn interpolate(&self, other: &GeoPosition, fraction: f64) -> GeoPosition {
        let fraction = fraction.clamp(0.0, 1.0);
        
        GeoPosition::new(
            self.latitude + (other.latitude - self.latitude) * fraction,
            self.longitude + (other.longitude - self.longitude) * fraction,
            self.altitude + (other.altitude - self.altitude) * fraction,
        )
    }

    /// Convert to (latitude, longitude) tuple
    pub fn to_tuple(&self) -> (f64, f64) {
        (self.latitude, self.longitude)
    }

    /// Convert to array [latitude, longitude, altitude]
    pub fn to_array(&self) -> [f64; 3] {
        [self.latitude, self.longitude, self.altitude]
    }
}

/// Geographic bounding box for area queries
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GeoBounds {
    pub min_lat: f64,
    pub max_lat: f64,
    pub min_lng: f64,
    pub max_lng: f64,
}

impl GeoBounds {
    pub fn new(min_lat: f64, max_lat: f64, min_lng: f64, max_lng: f64) -> Self {
        Self {
            min_lat,
            max_lat,
            min_lng,
            max_lng,
        }
    }

    /// Create bounds from center point and radius in kilometers
    pub fn from_center(center: &GeoPosition, radius_km: f64) -> Self {
        // Approximate - good enough for small areas
        let lat_delta = radius_km / 111.0; // ~111 km per degree latitude
        let lng_delta = radius_km / (111.0 * center.latitude.to_radians().cos());

        Self {
            min_lat: center.latitude - lat_delta,
            max_lat: center.latitude + lat_delta,
            min_lng: center.longitude - lng_delta,
            max_lng: center.longitude + lng_delta,
        }
    }

    /// Check if a position is within these bounds
    pub fn contains(&self, position: &GeoPosition) -> bool {
        position.latitude >= self.min_lat
            && position.latitude <= self.max_lat
            && position.longitude >= self.min_lng
            && position.longitude <= self.max_lng
    }

    /// Get the center of these bounds
    pub fn center(&self) -> GeoPosition {
        GeoPosition::new(
            (self.min_lat + self.max_lat) / 2.0,
            (self.min_lng + self.max_lng) / 2.0,
            0.0,
        )
    }
}

/// Geofence polygon for boundary checks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Geofence {
    pub name: String,
    pub vertices: Vec<GeoPosition>,
    pub max_altitude: Option<f64>,
}

impl Geofence {
    pub fn new(name: impl Into<String>, vertices: Vec<GeoPosition>) -> Self {
        Self {
            name: name.into(),
            vertices,
            max_altitude: None,
        }
    }

    /// Check if a position is inside this geofence using ray casting
    pub fn contains(&self, position: &GeoPosition) -> bool {
        if self.vertices.len() < 3 {
            return false;
        }

        // Check altitude if specified
        if let Some(max_alt) = self.max_altitude {
            if position.altitude > max_alt {
                return false;
            }
        }

        // Ray casting algorithm
        let mut inside = false;
        let n = self.vertices.len();
        let mut j = n - 1;

        for i in 0..n {
            let vi = &self.vertices[i];
            let vj = &self.vertices[j];

            if ((vi.longitude > position.longitude) != (vj.longitude > position.longitude))
                && (position.latitude
                    < (vj.latitude - vi.latitude) * (position.longitude - vi.longitude)
                        / (vj.longitude - vi.longitude)
                        + vi.latitude)
            {
                inside = !inside;
            }
            j = i;
        }

        inside
    }

    /// Get the bounding box of this geofence
    pub fn bounds(&self) -> GeoBounds {
        let min_lat = self.vertices.iter().map(|v| v.latitude).fold(f64::MAX, f64::min);
        let max_lat = self.vertices.iter().map(|v| v.latitude).fold(f64::MIN, f64::max);
        let min_lng = self.vertices.iter().map(|v| v.longitude).fold(f64::MAX, f64::min);
        let max_lng = self.vertices.iter().map(|v| v.longitude).fold(f64::MIN, f64::max);

        GeoBounds::new(min_lat, max_lat, min_lng, max_lng)
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distance_calculation() {
        // Kabul to Kandahar (approximately 450 km)
        let kabul = GeoPosition::new(34.5553, 69.2075, 0.0);
        let kandahar = GeoPosition::new(31.6133, 65.7101, 0.0);
        
        let distance = kabul.distance_to(&kandahar);
        assert!(distance > 400.0 && distance < 500.0);
    }

    #[test]
    fn test_bearing_calculation() {
        let origin = GeoPosition::new(0.0, 0.0, 0.0);
        let north = GeoPosition::new(1.0, 0.0, 0.0);
        let east = GeoPosition::new(0.0, 1.0, 0.0);

        let bearing_north = origin.bearing_to(&north);
        let bearing_east = origin.bearing_to(&east);

        assert!((bearing_north - 0.0).abs() < 1.0);
        assert!((bearing_east - 90.0).abs() < 1.0);
    }

    #[test]
    fn test_interpolation() {
        let start = GeoPosition::new(0.0, 0.0, 0.0);
        let end = GeoPosition::new(10.0, 10.0, 1000.0);

        let mid = start.interpolate(&end, 0.5);
        assert!((mid.latitude - 5.0).abs() < 0.01);
        assert!((mid.longitude - 5.0).abs() < 0.01);
        assert!((mid.altitude - 500.0).abs() < 0.01);
    }

    #[test]
    fn test_geo_bounds_contains() {
        let bounds = GeoBounds::new(30.0, 40.0, 60.0, 70.0);
        
        let inside = GeoPosition::new(35.0, 65.0, 0.0);
        let outside = GeoPosition::new(45.0, 75.0, 0.0);

        assert!(bounds.contains(&inside));
        assert!(!bounds.contains(&outside));
    }

    #[test]
    fn test_position_validity() {
        let valid = GeoPosition::new(45.0, 90.0, 1000.0);
        let invalid_lat = GeoPosition::new(100.0, 0.0, 0.0);
        let invalid_lng = GeoPosition::new(0.0, 200.0, 0.0);

        assert!(valid.is_valid());
        assert!(!invalid_lat.is_valid());
        assert!(!invalid_lng.is_valid());
    }
}
