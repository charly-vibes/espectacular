## 1. Bump dependency
- [ ] Bump `genesis` from `v0.1.0` to `v0.2.0` in `Cargo.toml`.

## 2. Adopt genesis::config
- [ ] If the tool has `src/config.rs`, thin it to just the struct + `ConfigFile` impl.
      Otherwise, add a minimal config struct implementing `ConfigFile`.
- [ ] Register the config struct with `ConfigRegistry` at startup.
- [ ] Remove dead config parsing code (if any).
- [ ] `cargo test` passes with the new config setup.

## 3. Adopt genesis::guide
- [ ] Replace `main.rs` CLI setup with `Guide::builder(...)`.
- [ ] Convert command handlers to return `Output<T>` and use `ErrorSink` for errors.
- [ ] Remove dead error-handling code.
- [ ] `cargo test` passes with the new guide setup.

## 4. Clean up
- [ ] `cargo test` passes.
- [ ] `cargo clippy` introduces no new warnings.
- [ ] `cargo fmt` is clean.