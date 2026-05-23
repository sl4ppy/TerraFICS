# Baseline: 2026-05-22, CREATIVE TEST.sav

**Machine:** 13th Gen Intel Core i9-13900K (24c/32t), 63.7 GB RAM, Windows 11 Pro (10.0.26200).
**Fixture:** `crates/scim-savefile/tests/corpus/CREATIVE TEST.sav` (1.35 MB compressed → 20.3 MB decompressed; save_version 46; 23,941 actors).
**Build profile:** `cargo bench` defaults to release (`opt-level=3`, `lto=thin`).
**Recorded at:** P1.2-c Task 8, immediately after `read_body` was parallelized via rayon.

## Numbers

| Bench | Median time | Notes |
|---|---|---|
| `read_header` | **150.79 ns** | Header is small (~459 bytes); allocation-dominated |
| `read_body[CREATIVE TEST.sav]` | **6.349 ms** | Parallel via rayon over 80 chunks |
| `read_body_envelope[CREATIVE TEST.sav]` | **525.86 µs** | 9,757 levels walked |
| `stream_actors[CREATIVE TEST.sav]` | **5.079 ms** | 23,941 actors iterated, no body decode |
| `parse_entity_body[CREATIVE TEST.sav, all actors]` | **98.73 ms** | Property bag decode for every actor — the heaviest leg |
| `import_save[CREATIVE TEST.sav]` | **4.203 s** | Full parse + SQLite import (blob hash + zstd compress + actor rows) |

## Notes

- The headline number is `parse_entity_body`: at ~99 ms we process all 23,941 actors and their property bags. Extrapolating linearly to a 500 MB save (~370× more actors): ~37 s. Well above the spec's 1.5 s budget — but that's the COLD path, and the dominant cost is per-actor string allocation in `read_property`. P1.2-d candidates: arena allocation, `Cow<'a, str>` for property names, lazy property bag decoding.

- `read_body` at 6.3 ms on a save that decompresses to 20 MB = ~3.2 GB/s decompression throughput. Well above raw single-core zlib (typically ~200 MB/s); the parallelism + small chunks help.

- `import_save` at 4.2 s is dominated by zstd compression of ~24k blobs. The dedup ratio (4.1×) means ~6k blobs actually get compressed. blake3 is fast enough to not show in the profile.

- These numbers establish a per-bench reference point. **Re-run on the SAME machine** when comparing PRs against this baseline; CI runners are too variable for direct comparison.

## Reproduce

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH
Push-Location D:\Projects\TerraFICS
cargo bench -p scim-savefile
cargo bench -p scim-store
Pop-Location
```

## P1.5-a — `scim-world` spatial index (added 2026-05-23)

Build + query of the R-tree over the actors in the CREATIVE TEST.sav snapshot.
Baseline machine: i9-13900K. The R-tree holds 17,974 entries (Object-kind rows
with NULL transforms are skipped — 5,967 of the 23,941 total actors are non-spatial).

| Bench | Low | Median | High |
|---|---|---|---|
| `scim_world::from_snapshot / CREATIVE TEST.sav` | 14.448 ms | 14.531 ms | 14.604 ms |
| `scim_world::query_aabb / viewport_100k`        | 6.6877 µs | 6.7163 µs | 6.7454 µs |

Notes:
- `from_snapshot` cost is dominated by the SQLite `SELECT … FROM actor` plus
  per-row transform decode. The `rstar::bulk_load` step is `O(n log n)` and is
  the smaller fraction. The bench measures the warm-DB case; cold first-load
  (immediately after `import_save`) costs ~63 ms because the WAL is still
  flushing — measured by the `from_snapshot_corpus` integration test.
- `query_aabb` returns an iterator; the bench `.count()`s it to force
  realization. At 6.7 µs per viewport query the per-frame cost is well under
  the 60 fps budget (16.7 ms) — the renderer's spatial-cull pass has plenty of
  headroom even at 1 GB+ saves with proportionally more placements.
