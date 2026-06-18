default:
    @just --list

# Install ah binary to ~/.cargo/bin
install:
    cargo install --path . --locked

prime:
    wai prime || wai status
    bd prime
    openspec list || true
    dont prime --plain || true

status:
    wai status
    bd ready
    openspec list || true
    dont list --plain || true

validate:
    openspec validate --all

plugins:
    wai plugin list
