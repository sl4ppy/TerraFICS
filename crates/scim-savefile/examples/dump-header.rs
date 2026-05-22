//! Smoke binary: print a `.sav` file's parsed header.
//!
//! Usage:
//!     cargo run -p scim-savefile --example dump-header -- path\to\save.sav

use std::fmt::Write as _;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: dump-header <path-to-sav>");
        return ExitCode::from(2);
    };
    let path = PathBuf::from(path);

    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) => { eprintln!("error: read {}: {e}", path.display()); return ExitCode::from(1); }
    };

    let (h, consumed) = match scim_savefile::read_header(&bytes) {
        Ok(x) => x,
        Err(e) => { eprintln!("error: parse header: {e}"); return ExitCode::from(1); }
    };

    println!("File:                {}", path.display());
    println!("Header bytes:        {consumed}");
    println!("save_header_type:    {}", h.save_header_type);
    println!("save_version:        {}", h.save_version);
    println!("build_version:       {}", h.build_version);
    if let Some(s) = &h.save_name        { println!("save_name:           {s}"); }
    println!("map_name:            {}", h.map_name);
    println!("map_options:         {}", h.map_options);
    println!("session_name:        {}", h.session_name);
    println!("play_duration_secs:  {}", h.play_duration_seconds);
    println!("save_date_time:      {}", h.save_date_time);
    println!("session_visibility:  {}", h.session_visibility);
    if let Some(v) = h.editor_object_version    { println!("editor_obj_version:  {v}"); }
    if let Some(m) = &h.mod_metadata            { println!("mod_metadata:        {m:?}"); }
    if let Some(v) = h.is_modded_save           { println!("is_modded_save:      {v}"); }
    if let Some(s) = &h.save_identifier         { println!("save_identifier:     {s}"); }
    if let Some(v) = h.is_partitioned_world     { println!("is_partitioned_world:{v}"); }
    if let Some(b) = &h.save_data_hash {
        let hex = b.iter().fold(String::with_capacity(b.len() * 2), |mut s, x| {
            write!(s, "{x:02x}").unwrap();
            s
        });
        println!("save_data_hash:      {hex}");
    }
    if let Some(v) = h.is_creative_mode_enabled { println!("is_creative_mode:    {v}"); }
    ExitCode::SUCCESS
}
