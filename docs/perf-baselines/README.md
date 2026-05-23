# Perf baselines

Committed snapshots of benchmark numbers — what the parse pipeline cost at the
time the snapshot was recorded. These are not enforced in CI; they're a
historical record that PR authors can compare against locally.

## How to record a new baseline

1. On a quiet machine (close browsers, IDEs, video calls):
   ```powershell
   $env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH
   Push-Location D:\Projects\TerraFICS
   cargo bench -p scim-savefile 2>&1 | Tee-Object new-baseline.txt
   cargo bench -p scim-store 2>&1 | Tee-Object -Append new-baseline.txt
   Pop-Location
   ```
2. Extract the `time:` lines and the machine info (CPU model, RAM, OS) into a
   new markdown file under `docs/perf-baselines/YYYY-MM-DD-<fixture>.md` using
   the existing files as templates.
3. Commit the file alongside the change that motivated the re-baselining (e.g.
   a milestone tag).

## What's measured

| Bench | Crate | Workload |
|---|---|---|
| `read_header` | scim-savefile | Header parse only |
| `read_body[CREATIVE TEST.sav]` | scim-savefile | Parallel chunk decompression |
| `read_body_envelope[CREATIVE TEST.sav]` | scim-savefile | Level + partition walk |
| `stream_actors[CREATIVE TEST.sav]` | scim-savefile | Full actor iteration (no body decode) |
| `parse_entity_body[CREATIVE TEST.sav, all actors]` | scim-savefile | Property bag decode for every actor |
| `import_save[CREATIVE TEST.sav]` | scim-store | Full parse + SQLite import |

## What's NOT measured (yet)

- 500 MB save (no fixture in repo; see `.gitattributes` for Git LFS setup).
- Memory (RSS) ceilings (spec budget: 1.5× compressed size).
- Cold-open project DB → first frame (no renderer yet, P1.5).
- Edit-to-render p95 (no edit path yet, P2).
- Diff snapshot perf (no diff API yet, P3).
