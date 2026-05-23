# Save fixture corpus

Pinned `.sav` files used by `scim-savefile` integration tests.

| File | Provenance | save_header_type | Notes |
|------|------------|------------------|-------|
| `CREATIVE TEST.sav` | Copied from AnthorNet/SC-InteractiveMap repo at commit 8f2277e | 13 | save_version=46, build_version=367502. ~1.35 MB sanity check. |

## Adding fixtures

1. Place the `.sav` here (or under `large/` if ≥ 100 MB — see below).
2. Add a row to the table above (run `cargo run --example dump-header -- <file>` to populate it).
3. Add a test case in `tests/header_corpus.rs`.

## Adding a large fixture (≥ 100 MB)

The repo is configured for Git LFS via `.gitattributes` so large `.sav` files
under `tests/corpus/large/` don't bloat clones.

### One-time per-clone setup

```powershell
git lfs install
```

### Adding a save

1. Verify `git lfs` is installed: `git lfs version` should print a version.
2. Drop the save into `crates/scim-savefile/tests/corpus/large/`. Conventional
   filenames: `late-game-1gb.sav`, `update8-modded.sav`, etc.
3. `git add` it — LFS auto-tracks it via the `.gitattributes` pattern. Verify
   with `git lfs ls-files`.
4. Commit + push. On push, the LFS layer uploads the binary content to a
   separate object store; only a small pointer file lands in the regular Git
   history.

Other contributors get the binary automatically on `git lfs pull` (or via
`GIT_LFS_SKIP_SMUDGE=1` if they want to skip download).

### Why not check in big files directly

A 500 MB `.sav` would balloon every clone. LFS adds a small pointer per file
to the regular history and stores the actual bytes separately.
