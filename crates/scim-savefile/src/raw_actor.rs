//! `RawActor` — the per-actor record produced by `stream_actors`. Pairs an object
//! header (className, transform, etc.) with its entity body bytes (property bag).
//!
//! Object headers and entity bodies are interleaved by index across the two blocks:
//! entity i corresponds to the i-th object header. `stream_actors` walks both blocks
//! in lockstep and yields one `RawActor` per index.

use crate::body_envelope::{BodyEnvelope, LevelInfo};
use crate::entity::{read_entities_in_level, RawEntity};
use crate::error::{Error, Result};
use crate::header::Header;
use crate::object_header::{read_objects_in_level, RawObjectHeader};

#[derive(Debug)]
pub struct RawActor<'a> {
    pub level_name: &'a str,
    pub header: RawObjectHeader,
    pub entity: RawEntity<'a>,
}

/// Iterate over every (`object_header`, entity) pair across all levels in `env`.
///
/// Yields `Err` on any parse failure; iteration continues to the next level after a
/// level-level error so a single bad level does not abort the whole stream.
pub fn stream_actors<'a>(
    env: &'a BodyEnvelope<'a>,
    header: &'a Header,
) -> impl Iterator<Item = Result<RawActor<'a>>> + 'a {
    let map_name = header.map_name.clone();
    env.levels
        .iter()
        .flat_map(move |level| match actors_for_level(env, level, &map_name) {
            Ok(items) => items,
            Err(e) => vec![Err(e)],
        })
}

/// Collect a single level's actors as a Vec of per-actor `Result`s.
/// Returns `Err` if either block (objects or entities) fails to parse, or if
/// their counts disagree.
fn actors_for_level<'a>(
    env: &'a BodyEnvelope<'a>,
    level: &'a LevelInfo,
    map_name: &str,
) -> Result<Vec<Result<RawActor<'a>>>> {
    let objects_slice = &env.body_bytes[level.objects_byte_range.clone()];
    let entities_slice = &env.body_bytes[level.entities_byte_range.clone()];

    let headers = read_objects_in_level(objects_slice, level.save_version, map_name)?;
    let entities = read_entities_in_level(entities_slice)?;

    if headers.len() != entities.len() {
        return Err(Error::ObjectEntityCountMismatch {
            level: level.name.clone(),
            objects: headers.len(),
            entities: entities.len(),
        });
    }

    let level_name = level.name.as_str();
    Ok(headers
        .into_iter()
        .zip(entities)
        .map(move |(h, e)| {
            Ok(RawActor {
                level_name,
                header: h,
                entity: e,
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body_envelope::LevelInfo;

    fn synth_test_header() -> Header {
        Header {
            save_header_type: 14,
            save_version: 46,
            build_version: 0,
            save_name: None,
            map_name: "Foo".to_string(),
            map_options: String::new(),
            session_name: String::new(),
            play_duration_seconds: 0,
            save_date_time: 0,
            session_visibility: 0,
            editor_object_version: None,
            mod_metadata: None,
            is_modded_save: None,
            save_identifier: None,
            is_partitioned_world: Some(0),
            save_data_hash: None,
            is_creative_mode_enabled: None,
        }
    }

    #[test]
    fn empty_level_yields_no_actors() {
        // A level with empty objects and entities ranges (just two i32-zero counts)
        // should produce no actors.
        let body = vec![0_u8; 16];
        let env = BodyEnvelope {
            partitions: None,
            levels: vec![LevelInfo {
                name: "Level Foo".to_string(),
                save_version: 46,
                objects_byte_range: 0..4,
                entities_byte_range: 4..8,
            }],
            body_bytes: &body,
        };
        let header = synth_test_header();
        assert_eq!(stream_actors(&env, &header).count(), 0);
    }
}
