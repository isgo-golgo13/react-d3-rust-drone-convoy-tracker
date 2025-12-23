//! Database migrations
//!
//! Handles schema versioning and migrations for ScyllaDB.

use crate::{DbError, DbResult};
use scylla::Session;
use std::sync::Arc;
use tracing::{info, warn};

/// Run all pending migrations
pub async fn run_all(session: &Arc<Session>) -> DbResult<()> {
    info!("🔄 Running database migrations...");

    // Check if migrations table exists
    ensure_migrations_table(session).await?;

    // Get applied migrations
    let applied = get_applied_migrations(session).await?;

    // Run pending migrations
    let migrations = get_migrations();
    let mut applied_count = 0;

    for (version, name, cql) in migrations {
        if !applied.contains(&version) {
            info!("  Applying migration {}: {}", version, name);
            apply_migration(session, version, name, cql).await?;
            applied_count += 1;
        }
    }

    if applied_count == 0 {
        info!("✅ No pending migrations");
    } else {
        info!("✅ Applied {} migrations", applied_count);
    }

    Ok(())
}

/// Ensure migrations tracking table exists
async fn ensure_migrations_table(session: &Arc<Session>) -> DbResult<()> {
    let query = r#"
        CREATE TABLE IF NOT EXISTS schema_migrations (
            version INT PRIMARY KEY,
            name TEXT,
            applied_at TIMESTAMP
        )
    "#;

    session
        .query_unpaged(query, &[])
        .await
        .map_err(|e| DbError::Migration(e.to_string()))?;

    Ok(())
}

/// Get list of applied migration versions
async fn get_applied_migrations(session: &Arc<Session>) -> DbResult<Vec<i32>> {
    let query = "SELECT version FROM schema_migrations";
    
    let result = session
        .query_unpaged(query, &[])
        .await
        .map_err(|e| DbError::Migration(e.to_string()))?;

    let mut versions = Vec::new();
    if let Some(rows) = result.rows {
        for row in rows {
            if let Ok(version) = row.columns[0].as_ref()
                .and_then(|v| v.as_int())
                .ok_or_else(|| DbError::Migration("Invalid version".into())) 
            {
                versions.push(version);
            }
        }
    }

    Ok(versions)
}

/// Apply a single migration
async fn apply_migration(
    session: &Arc<Session>,
    version: i32,
    name: &str,
    cql: &str,
) -> DbResult<()> {
    // Execute migration
    for statement in cql.split(';').filter(|s| !s.trim().is_empty()) {
        session
            .query_unpaged(statement.trim(), &[])
            .await
            .map_err(|e| DbError::Migration(format!("Migration {} failed: {}", version, e)))?;
    }

    // Record migration
    let record_query = r#"
        INSERT INTO schema_migrations (version, name, applied_at)
        VALUES (?, ?, toTimestamp(now()))
    "#;

    session
        .query_unpaged(record_query, (version, name))
        .await
        .map_err(|e| DbError::Migration(e.to_string()))?;

    Ok(())
}

/// Get all migrations in order
fn get_migrations() -> Vec<(i32, &'static str, &'static str)> {
    vec![
        (1, "create_drone_telemetry_index", r#"
            CREATE INDEX IF NOT EXISTS idx_telemetry_mission 
            ON drone_telemetry (mission_id)
        "#),
        (2, "create_alerts_severity_index", r#"
            CREATE INDEX IF NOT EXISTS idx_alerts_severity 
            ON alerts (severity)
        "#),
        (3, "create_cv_tracking_confidence_index", r#"
            CREATE INDEX IF NOT EXISTS idx_cv_confidence 
            ON cv_tracking (confidence)
        "#),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migrations_ordered() {
        let migrations = get_migrations();
        let mut last_version = 0;
        
        for (version, _, _) in migrations {
            assert!(version > last_version, "Migrations must be ordered by version");
            last_version = version;
        }
    }
}
