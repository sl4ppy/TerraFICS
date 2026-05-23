//! `Component` trait — typed per-class view of a `RawActor`.
//!
//! Per design spec §4.2:
//! - `decode(&RawActor, &EntityBody) -> Result<Self>`: reconstruct the typed
//!   view from raw bytes.
//! - `encode_into(&self, &mut Vec<u8>) -> Result<()>`: serialize back to the
//!   original wire format. Used by `scim-store` to round-trip edits.
//! - `affected_indices(&self) -> &[usize]`: spatial-index hint — which world
//!   cells does this actor occupy, expressed as the indices into the
//!   `scim-world` cell grid (P2 wires this up; P1.3-c returns an empty slice).

use scim_savefile::{EntityBody, RawActor};

use crate::error::Result;

pub trait Component: Sized {
    fn decode(raw_actor: &RawActor<'_>, body: &EntityBody<'_>) -> Result<Self>;

    fn encode_into(&self, out: &mut Vec<u8>) -> Result<()>;

    fn affected_indices(&self) -> &[usize] {
        &[]
    }
}
