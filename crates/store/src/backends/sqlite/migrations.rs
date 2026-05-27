//! SQLite migration runner.
//!
//! Custom minimal runner for 01e. Tradeoff: `refinery` is mature and handles
//! down-migrations + checksums, but 01e has only one migration file. Revisit
//! if migrations exceed 3–5 files or if multi-backend orchestration is needed.

use rusqlite::Connection;

/// Migration record.
#[derive(Debug, Clone)]
pub struct MigrationRecord {
    pub version: i64,
    pub name: String,
    pub checksum: String,
    pub dirty: bool,
    pub applied_at: i64,
}

/// Run all pending migrations. Creates the `openwand_migration` table if needed.
/// Blocks on `dirty` flag (crash recovery signal).
pub fn run_migrations(conn: &Connection) -> Result<(), String> {
    // Create migration tracking table
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS openwand_migration (
            version     INTEGER PRIMARY KEY,
            name        TEXT NOT NULL,
            checksum    TEXT NOT NULL,
            dirty       INTEGER NOT NULL DEFAULT 0,
            applied_at  INTEGER NOT NULL
        );",
    )
    .map_err(|e| format!("create migration table: {e}"))?;

    // Check for dirty flag
    let dirty: bool = conn
        .query_row(
            "SELECT COALESCE(MAX(dirty), 0) FROM openwand_migration",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|v| v != 0)
        .unwrap_or(false);

    if dirty {
        return Err(
            "Database has a dirty migration flag. Manual recovery required.".into()
        );
    }

    // Get current version
    let current_version: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM openwand_migration",
            [],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0);

    // Run migration 0001 if not yet applied
    let migrations: &[(i64, &str, &str, &str)] = &[
        (
            1,
            "0001_trace",
            crate::backends::sqlite::schema::MIGRATION_0001_CHECKSUM,
            crate::backends::sqlite::schema::MIGRATION_0001_SQL,
        ),
    ];

    for &(version, name, checksum, sql) in migrations {
        if version <= current_version {
            continue;
        }

        // Mark dirty before applying
        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "INSERT INTO openwand_migration (version, name, checksum, dirty, applied_at) VALUES (?1, ?2, ?3, 1, ?4)",
            rusqlite::params![version, name, checksum, now],
        )
        .map_err(|e| format!("mark dirty v{version}: {e}"))?;

        // Apply
        conn.execute_batch(sql)
            .map_err(|e| format!("apply migration v{version}: {e}"))?;

        // Clear dirty flag
        conn.execute(
            "UPDATE openwand_migration SET dirty = 0 WHERE version = ?1",
            rusqlite::params![version],
        )
        .map_err(|e| format!("clear dirty v{version}: {e}"))?;
    }

    Ok(())
}

/// Verify that all required tables exist.
pub fn verify_schema(conn: &Connection) -> Result<bool, String> {
    let required = ["trace_entry", "trace_relation", "trace_blob", "openwand_migration"];
    for table in &required {
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name=?1",
                rusqlite::params![table],
                |row| row.get::<_, bool>(0),
            )
            .unwrap_or(false);
        if !exists {
            return Ok(false);
        }
    }
    Ok(true)
}
