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

## P1.5-b — `scim-render` foundation (added 2026-05-23)

First GPU code in the project. The footprint pass draws one instanced unit
quad per actor placement; no criterion bench (wgpu has no headless story we
want to invest in for v1). Numbers below are observational from the viewer
example on the baseline machine.

| Metric | Observation |
|---|---|
| Instance buffer build (`build_instances` over 17,974 placements) | < 1 ms — trivial Vec collect |
| Renderer construction (`Renderer::new` async, includes adapter + device + pipeline) | ~100–300 ms cold (driver init) |
| Per-frame render (17,974 instanced quads, 1280×800 surface) | 60 FPS locked with v-sync; well under 16.7 ms budget |
| Cold launch → first frame (`import_save` → `WorldIndex` → upload → render) | ~5–6 s, dominated by `import_save` (~5 s for CREATIVE TEST.sav) |

Notes:
- The 60 FPS observation is from v-sync-bound rendering. The actual GPU work
  per frame is negligible (under 1 ms) — measured indirectly by hardware
  monitor tools, not in-code. Real perf budget validation (spec §6.6: 60 FPS
  at 1440p with ~1M actors) waits on a larger fixture.
- `import_save` still dominates cold launch. The spec budget is "Cold load
  .sav → first interactive frame: < 2 s" at 500 MB. Today's import takes
  ~5 s on the 1.35 MB CREATIVE TEST.sav; that's an O(n) cost that will
  dominate any future 500 MB save. P1.2-d perf v2 addresses this — it
  remains the open perf debt.
- The renderer does NOT yet implement viewport culling (R-tree
  `query_aabb` is built and benched in P1.5-a but the renderer uploads ALL
  placements every frame). With 17,974 instances on a desktop GPU this is
  fine; at 1M+ it will become necessary. Culling lands when the spec §6.6
  perf headroom requires it — earliest in P1.5-c (picking shares the
  query infrastructure), latest if perf bites.

## P1.5-c — picking + selection (added 2026-05-23)

GPU click-pick + flag-bit selection highlight. No criterion bench (same
reason as P1.5-b: no headless wgpu story we want to invest in for v1).
Observational from the viewer on the baseline machine (i9-13900K + dedicated
GPU, 17,974 instances, 1280×800 surface).

| Metric | Observation |
|---|---|
| Click → tint round trip (renderer.pick + set_selection + redraw) | < 50 ms wall clock; visually instantaneous |
| renderer.pick alone (encode + submit + map_async + device.poll(Wait)) | A few ms; bounded by GPU work + driver readback latency |
| renderer.set_selection (two 4-byte queue.write_buffer calls) | Negligible (microseconds-scale) |
| Pick texture memory | width × height × 4 B (~4 MB at 1280×800; ~12 MB at 1440p) — trivial |
| Staging buffer | 256 B (one padded row for a single u32 readback) |

Notes:
- The blocking `device.poll(Wait)` is acceptable for click but NOT for hover.
  Hover lands in a separate milestone with ring-buffered staging + async
  poll, matching spec §6.3.
- Scissored 1×1 pick pass means the rasteriser only colours one pixel per
  click regardless of instance count. The dominant cost is the GPU submit +
  driver-side readback latency, which is roughly fixed.
- `actor_ids` linear scan in `set_selection` is O(n) over the instance
  count (~18k at the corpus scale). At 18k that's negligible; if hover
  hammers it 60 times per second at 1M instances, swap for a `HashMap<i64, usize>`
  populated alongside `upload_world`. Documented; not changed in P1.5-c.

## P1.5-d — base map tiles (added 2026-05-26)

PNG tile pyramid rendered under the actor footprints; alpha-blended
footprints (0.65 base / 0.85 selected). Local files only — no CDN fetch
in this milestone. Same wgpu-no-headless policy as P1.5-b/c: numbers below
are observational from the viewer on the baseline machine (i9-13900K +
dedicated GPU) with real Test_01.sav and SCIM's "Stable"-build z=3..z=5
tiles fetched via scripts/fetch-tiles.ps1.

| Metric | Observation |
|---|---|
| Zoom-selection + visible-tile enumeration per frame | < 10 µs at typical visible-tile counts |
| First-frame-with-tiles latency (cold launch, no resident tiles) | 1–3 frames; viewer redraws while tiles are streaming in (tiles_loading() loop) |
| Tile decode (PNG → RGBA8) on background thread | A few ms per 256×256 tile; bounded by `image 0.25`'s PNG decoder |
| Tile GPU upload (`queue.write_texture`, 256 KB per tile) | Negligible (microseconds) |
| Visible-and-resident tile draw (one bind-per-tile + 6 indices) | tens of tiles per frame; well under the 16.7 ms budget at 60 FPS |
| Per-frame VRAM (256 resident tiles × 256² × 4 B) | 64 MB — bounded by `MAX_RESIDENT_TILES` |
| Alpha-blending the footprint pass | Negligible (fixed-function blend over ~17k visible quads) |

Notes:
- LRU eviction is O(n log n) on the sort but only runs when the cache is
  over budget; ~256 entries makes this trivial.
- Loader-thread request channel is unbounded; pathological camera paths
  could back it up but no real workload reaches that.
- The blocking `device.poll(Wait)` in `Renderer::pick` (P1.5-c) is unaffected
  by tiles — picking operates on the footprint pipeline only, and tiles
  don't enter the R32_UINT pick pass.
- Tile pyramid math: at z=N, each tile is `TILE_PYRAMID_SIZE / 2^N` game
  units wide, where `TILE_PYRAMID_SIZE = 1_500_001` (derived from SCIM's
  `scale(zoomRatio) * xMax / backgroundSize`). Pyramid extends past the
  displayed bounds; tiles outside the playable area 404 on the CDN and are
  cached as `failed` to avoid re-requests.
