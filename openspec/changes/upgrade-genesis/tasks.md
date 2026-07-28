## 1. Bump dependency
- [x] Bump `genesis` from `v0.1.0` to `v0.2.0` in `Cargo.toml`.

## 2. Adopt genesis::config
- [x] If the tool has `src/config.rs`, thin it to just the struct + `ConfigFile` impl.
      Otherwise, add a minimal config struct implementing `ConfigFile`.
- [x] Register the config struct with `ConfigRegistry` at startup.
- [x] Remove dead config parsing code (if any).
- [x] `cargo test` passes with the new config setup.

## 3. Adopt genesis::guide
- [x] Replace `main.rs` CLI setup with `Guide::builder(...)`.
- [x] Convert command handlers to return `Output<T>` and use `ErrorSink` for errors.
- [x] Remove dead error-handling code.
- [x] `cargo test` passes with the new guide setup.

## 4. Clean up
- [x] `cargo test` passes.
- [x] `cargo clippy` introduces no new warnings.
- [x] `cargo fmt` is clean.
