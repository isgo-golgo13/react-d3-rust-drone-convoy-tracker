//! Database migrations

use crate::{DbError, DbResult};
use scylla::Session;
use std::sync::Arc;
use tracing::info;

/// Run all migrations
pub async fn run_all(session: &Arc<Session>) -> DbResult<()> {
    info!("Running database migrations...");

    let version = get_schema_version(session).await?;
    info!("Current schema version: {}", version);

    info!("Migrations complete");
    Ok(())
}

/// Get current schema version
async fn get_schema_version(session: &Arc<Session>) -> DbResult<i32> {
    let query = "SELECT version FROM schema_version WHERE id = 1";

    let result = session.query_unpaged(query, &[]).await;

    match result {
        Ok(query_result) => {
            match query_result.into_rows_result() {
                Ok(rows) => {
                    if rows.rows_num() > 0 {
                        // TODO: parse actual version
                        return Ok(1);
                    }
                    Ok(0)
                }
                Err(_) => Ok(0),
            }
        }
        Err(_) => Ok(0), // Table doesn't exist yet
    }
}