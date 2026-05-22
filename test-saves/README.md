# Live save fixtures

Drop any number of your own Satisfactory `.sav` files into this directory.

When `cargo test` runs, the integration test in `crates/scim-savefile/tests/live_saves.rs` will discover them and exercise the parser against each one. If this folder is empty, the test no-ops (it prints a notice and passes).

**Privacy:** files in this directory are `.gitignore`'d — they will never be committed. Save files contain personally identifiable info (session names, save identifiers, mod metadata). Keep them local.

## How to use

1. Copy any `.sav` file into this directory:

```powershell
Copy-Item "$env:LOCALAPPDATA\FactoryGame\Saved\SaveGames\<your-save>.sav" D:\Projects\TerraFICS\test-saves\
```

2. Run the integration test:

```powershell
cd D:\Projects\TerraFICS
cargo test -p scim-savefile --test live_saves -- --nocapture
```

The `--nocapture` flag lets you see the per-file header summary and body expansion ratio.

## What the test checks

For each `.sav` found:

- Header parses cleanly (`save_header_type` is in supported range)
- If `save_version >= 41`: body decompresses and produces a non-empty `Vec<u8>` with a plausible leading length prefix
- If `save_version < 41`: the test prints a note and skips (older formats aren't supported yet — not a failure)

The test FAILS only if a save in the supported version range produces a parser error.

## Cleanup

Remove a fixture any time:

```powershell
Remove-Item D:\Projects\TerraFICS\test-saves\<save-name>.sav
```
