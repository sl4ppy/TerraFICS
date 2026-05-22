# Save fixture corpus

Pinned `.sav` files used by `scim-savefile` integration tests.

| File | Provenance | save_header_type | Notes |
|------|------------|------------------|-------|
| `CREATIVE TEST.sav` | Copied from AnthorNet/SC-InteractiveMap repo at commit 8f2277e | TBD (read on first test run; document here) | Small ~1.6 MB sanity check. |

## Adding fixtures

1. Place the `.sav` here.
2. Add a row to the table above (run `cargo run --example dump-header -- <file>` to populate it).
3. Add a test case in `tests/header_corpus.rs`.
4. If the file is >10 MB, switch it to Git LFS first: `git lfs track "tests/corpus/<file>"`.
