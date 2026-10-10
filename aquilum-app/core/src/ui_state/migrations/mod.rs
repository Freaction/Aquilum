pub mod schema;
pub mod v1_to_v2;
pub mod v2_to_v3;
pub mod v3_to_v4;
pub mod v4_to_v5;
pub mod v5_to_v6;
pub mod v6_to_v7;
pub mod v7_to_v8;
pub mod v8_to_v9;

use super::error::UiStateError;
use rusqlite::Connection;
use schema::SCHEMA;
use v1_to_v2::migrate_v1_to_v2;
use v2_to_v3::migrate_v2_to_v3;
use v3_to_v4::migrate_v3_to_v4;
use v4_to_v5::migrate_v4_to_v5;
use v5_to_v6::migrate_v5_to_v6;
use v6_to_v7::migrate_v6_to_v7;
use v7_to_v8::migrate_v7_to_v8;
use v8_to_v9::migrate_v8_to_v9;

type Step = fn(&Connection) -> Result<(), UiStateError>;

const STEPS: [Step; 8] = [
    migrate_v1_to_v2,
    migrate_v2_to_v3,
    migrate_v3_to_v4,
    migrate_v4_to_v5,
    migrate_v5_to_v6,
    migrate_v6_to_v7,
    migrate_v7_to_v8,
    migrate_v8_to_v9,
];

const LATEST_VERSION: i64 = STEPS.len() as i64 + 1;

pub fn migrate(connection: &Connection) -> Result<(), UiStateError> {
    connection.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA busy_timeout = 2000;",
    )?;
    let version = connection.query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))?;
    let from = match version {
        0 => {
            create_latest(connection)?;
            LATEST_VERSION
        }
        1..=LATEST_VERSION => version,
        version => return Err(UiStateError::UnsupportedSchema { version }),
    };
    for step in &STEPS[(from - 1) as usize..] {
        step(connection)?;
    }
    Ok(())
}

fn create_latest(connection: &Connection) -> Result<(), UiStateError> {
    connection.execute_batch("PRAGMA auto_vacuum = INCREMENTAL;")?;
    connection.execute_batch(&format!(
        "BEGIN IMMEDIATE; {SCHEMA} PRAGMA user_version = {LATEST_VERSION}; COMMIT;"
    ))?;
    Ok(())
}
