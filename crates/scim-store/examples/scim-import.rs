//! Smoke binary: import a `.sav` into a project `SQLite` DB.
//!
//! Usage:
//!     cargo run -p scim-store --example scim-import -- path\to\save.sav path\to\project.scimdb

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use scim_store::{import::import_save, Db};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(sav) = args.next() else {
        eprintln!("usage: scim-import <save.sav> <project.scimdb>");
        return ExitCode::from(2);
    };
    let Some(db_path) = args.next() else {
        eprintln!("usage: scim-import <save.sav> <project.scimdb>");
        return ExitCode::from(2);
    };

    let sav = PathBuf::from(sav);
    let db_path = PathBuf::from(&db_path);

    let mut db = match Db::open(&db_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: open db {}: {e}", db_path.display());
            return ExitCode::from(1);
        }
    };

    let label = sav
        .file_name()
        .map_or_else(|| "unnamed".to_string(), |s| s.to_string_lossy().into_owned());

    let t0 = Instant::now();
    let summary = match import_save(&mut db, &sav, &label) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: import: {e}");
            return ExitCode::from(1);
        }
    };
    let elapsed = t0.elapsed();

    println!("Imported {}", sav.display());
    println!("  snapshot_id:     {}", summary.snapshot_id);
    println!("  total actors:    {}", summary.total_actors);
    println!("  unique blobs:    {}", summary.blobs_inserted);
    println!("  failed actors:   {}", summary.failed_actors);
    println!("  elapsed:         {:.2}s", elapsed.as_secs_f64());

    ExitCode::SUCCESS
}
