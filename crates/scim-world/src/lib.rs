//! In-memory world graph and spatial index over a `scim-store` snapshot.
//!
//! Per design spec §6.2: a persistent R-tree (`rstar`) materialized from the
//! store on snapshot load. The renderer (P1.5-b+) queries it per frame with
//! the visible AABB; picking (P1.5-c) is the same query at a point.
//!
//! P1.5-a delivers the read-only materialization + queries. Incremental edit
//! updates land with the P2 edit path.

pub mod error;
pub use error::{Error, Result};
pub mod placement;
pub use placement::ActorPlacement;
pub mod index;
pub use index::WorldIndex;
