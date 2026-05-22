# TerraFICS

Native Rust reimplementation of [SCIM](https://github.com/AnthorNet/SC-InteractiveMap) — the Satisfactory save editor and map viewer — targeting large saves, persistent snapshot/diff history, and Lua scripting.

**Status:** Phase 1.1 (walking skeleton). Not yet usable.

**Design doc:** see the parent repo's `docs/superpowers/specs/2026-05-22-scim-native-port-design.md`.

## Build

```
cargo build --release
```

## Run the smoke binary

```
cargo run -p scim-savefile --example dump-header -- path\to\save.sav
```

## License

Reuse of source code and data assets is not permitted in any case. Source is available for educational purposes only.
