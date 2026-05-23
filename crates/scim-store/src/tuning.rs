//! `SQLite` pragma tuning. Per design spec §5.7: without these, a 1 GB import
//! takes 5+ minutes; with them, it's 3-5 seconds.

use rusqlite::Connection;

use crate::error::Result;

pub fn apply_tuning(conn: &Connection) -> Result<()> {
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "journal_size_limit", 67_108_864_i64)?;
    conn.pragma_update(None, "cache_size", -65_536_i64)?;
    conn.pragma_update(None, "temp_store", "MEMORY")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_tuning_to_in_memory_db() {
        let conn = Connection::open_in_memory().unwrap();
        apply_tuning(&conn).unwrap();
    }
}
