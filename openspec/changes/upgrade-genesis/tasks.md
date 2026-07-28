## 1. Bump dependency
- [ ] Bump `genesis` from `v0.1.0` to `v0.2.0` in `Cargo.toml`.

## 2. Adopt genesis::config
- [ ] Thin `src/config.rs` to just the struct + `ConfigFile` impl.
- [ ] Register with `ConfigRegistry` at startup.
- [ ] Remove dead config parsing code.
- [ ] Tests pass with the new config setup.

## 3. Adopt genesis::guide (optional)
- [ ] Replace `main.rs` CLI setup with `Guide::builder(...)`.
- [ ] Convert command handlers to return `Output<T>`, `ErrorSink` for errors.
- [ ] Remove dead error-handling code.
- [ ] Tests pass with the new guide setup.

## 4. Clean up
- [ ] Remove unused genesis imports.
- [ ] `cargo test` passes.
- [ ] `cargo clippy` clean.
- [ ] `cargo fmt` clean.
