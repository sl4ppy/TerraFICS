//! Smoke binary: print a `.sav` file's parsed header.
//!
//! Usage:
//!     cargo run -p scim-savefile --example dump-header -- path\to\save.sav

use std::fmt::Write as _;
use std::path::PathBuf;
use std::process::ExitCode;

#[allow(clippy::too_many_lines)] // sequential diagnostic pipeline; splitting would obscure flow
fn main() -> ExitCode {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: dump-header <path-to-sav>");
        return ExitCode::from(2);
    };
    let path = PathBuf::from(path);

    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: read {}: {e}", path.display());
            return ExitCode::from(1);
        }
    };

    let (h, consumed) = match scim_savefile::read_header(&bytes) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("error: parse header: {e}");
            return ExitCode::from(1);
        }
    };

    println!("File:                {}", path.display());
    println!("Header bytes:        {consumed}");
    println!("save_header_type:    {}", h.save_header_type);
    println!("save_version:        {}", h.save_version);
    println!("build_version:       {}", h.build_version);
    if let Some(s) = &h.save_name {
        println!("save_name:           {s}");
    }
    println!("map_name:            {}", h.map_name);
    println!("map_options:         {}", h.map_options);
    println!("session_name:        {}", h.session_name);
    println!("play_duration_secs:  {}", h.play_duration_seconds);
    println!("save_date_time:      {}", h.save_date_time);
    println!("session_visibility:  {}", h.session_visibility);
    if let Some(v) = h.editor_object_version {
        println!("editor_obj_version:  {v}");
    }
    if let Some(m) = &h.mod_metadata {
        println!("mod_metadata:        {m:?}");
    }
    if let Some(v) = h.is_modded_save {
        println!("is_modded_save:      {v}");
    }
    if let Some(s) = &h.save_identifier {
        println!("save_identifier:     {s}");
    }
    if let Some(v) = h.is_partitioned_world {
        println!("is_partitioned_world:  {v}");
    }
    if let Some(b) = &h.save_data_hash {
        let hex = b
            .iter()
            .fold(String::with_capacity(b.len() * 2), |mut s, x| {
                write!(s, "{x:02x}").unwrap();
                s
            });
        println!("save_data_hash:      {hex}");
    }
    if let Some(v) = h.is_creative_mode_enabled {
        println!("is_creative_mode:    {v}");
    }

    // Decompress the body so we can also report its size.
    match scim_savefile::read_body(&bytes[consumed..], h.save_version) {
        Ok(body) => {
            println!("body_compressed:     {} bytes", bytes.len() - consumed);
            println!("body_decompressed:   {} bytes", body.len());
            let compressed = bytes.len() - consumed;
            #[allow(clippy::cast_precision_loss)] // display-only ratio
            let ratio = body.len() as f64 / compressed.max(1) as f64;
            println!("expansion_ratio:     {ratio:.2}x");

            // Walk the envelope + stream actors for diagnostic output.
            match scim_savefile::read_body_envelope(&body, &h) {
                Ok(env) => {
                    println!("levels:              {}", env.levels.len());
                    let mut total = 0_usize;
                    let mut prop_total = 0_usize;
                    let mut fully = 0_usize;
                    for r in scim_savefile::stream_actors(&env, &h) {
                        let Ok(actor) = r else { continue };
                        total += 1;
                        if h.save_version < 53 {
                            let lvl_sv = env
                                .levels
                                .iter()
                                .find(|l| l.name == actor.level_name)
                                .map_or(h.save_version, |l| l.save_version);
                            if let Ok(eb) = scim_savefile::parse_entity_body(
                                &actor, lvl_sv, 1000, &h.map_name,
                            ) {
                                prop_total += eb.properties.len();
                                if eb.first_unsupported.is_none() {
                                    fully += 1;
                                }
                            }
                        }
                    }
                    println!("actors:              {total}");
                    println!("properties:          {prop_total}");
                    println!("fully_parsed_actors: {fully}");
                }
                Err(e) => eprintln!("warning: envelope: {e}"),
            }
        }
        Err(e) => {
            eprintln!("warning: body decompression failed: {e}");
            // Don't fail the example for a body issue — the header still parsed.
        }
    }

    ExitCode::SUCCESS
}
