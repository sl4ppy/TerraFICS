//! `import_save` — open a `.sav`, parse it, write every actor into the database
//! as a new snapshot. Single SQL transaction; reverts cleanly on any error.

use std::collections::HashSet;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use scim_savefile::{
    read_body, read_body_envelope, read_header, stream_actors, ObjectHeaderBody,
};

use crate::actor::{encode_transform, insert_actor};
use crate::blob::insert_blob_if_absent;
use crate::db::Db;
use crate::error::Result;
use crate::header_store::insert_header;
use crate::snapshot::{add_actor_to_snapshot, create_snapshot};

/// Stats from an import.
#[derive(Debug, Clone, Copy)]
pub struct ImportSummary {
    pub snapshot_id: i64,
    pub total_actors: usize,
    pub blobs_inserted: usize,
    pub failed_actors: usize,
}

/// Open `sav_path`, parse, import into `db` as a fresh snapshot.
#[allow(clippy::cast_possible_wrap)]
pub fn import_save<P: AsRef<Path>>(
    db: &mut Db,
    sav_path: P,
    label: &str,
) -> Result<ImportSummary> {
    let sav_path = sav_path.as_ref();
    let bytes = std::fs::read(sav_path)?;
    let (header, consumed) = read_header(&bytes)?;
    let body = read_body(&bytes[consumed..], header.save_version)?;
    let env = read_body_envelope(&body, &header)?;

    let created_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let sav_path_str = sav_path.to_string_lossy().into_owned();

    let tx = db.conn_mut().transaction()?;

    let snapshot_id = create_snapshot(
        &tx,
        None,
        created_at,
        label,
        None,
        Some(sav_path_str.as_str()),
    )?;
    insert_header(&tx, snapshot_id, &header)?;

    let mut total_actors = 0_usize;
    let mut blobs_inserted = 0_usize;
    let mut failed_actors = 0_usize;
    let mut seen_hashes: HashSet<[u8; 32]> = HashSet::new();

    for r in stream_actors(&env, &header) {
        let Ok(actor) = r else {
            failed_actors += 1;
            continue;
        };
        let body_bytes = actor.entity.body_bytes;
        let hash = insert_blob_if_absent(&tx, body_bytes)?;
        if seen_hashes.insert(*hash.as_array()) {
            blobs_inserted += 1;
        }

        let transform: Option<[u8; 40]> = match &actor.header.body {
            ObjectHeaderBody::Actor { transform } => Some(encode_transform(
                transform.rotation,
                transform.translation,
                transform.scale3d,
            )),
            ObjectHeaderBody::Object { .. } => None,
        };

        let actor_id = insert_actor(
            &tx,
            &actor.header.reference.path_name,
            &actor.header.class_name,
            actor.level_name,
            transform.as_ref(),
            hash,
        )?;
        add_actor_to_snapshot(&tx, snapshot_id, actor_id)?;
        total_actors += 1;
    }

    tx.commit()?;

    Ok(ImportSummary {
        snapshot_id,
        total_actors,
        blobs_inserted,
        failed_actors,
    })
}
