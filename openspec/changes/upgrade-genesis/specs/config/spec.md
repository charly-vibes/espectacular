# config spec delta: adopt genesis::config

## ADDED Requirements

### Requirement: Shared config management

The tool SHALL adopt `genesis::config` for shared config management.

#### Scenario: config struct implements ConfigFile

- **GIVEN** the tool has a config struct
- **WHEN** the struct implements `genesis::config::ConfigFile`
- **THEN** the tool's `src/config.rs` SHALL be reduced to the struct + `ConfigFile` impl
- **AND** all config file I/O (read, write, parse) SHALL delegate to genesis

#### Scenario: config registered at startup

- **GIVEN** the tool starts up
- **WHEN** it initializes
- **THEN** it SHALL register its config struct with `ConfigRegistry`
- **AND** it SHALL use `ConfigStore` for config discovery and validation

#### Scenario: dead config code removed

- **GIVEN** the tool has adopted genesis::config
- **WHEN** the old config parsing code is removed
- **THEN** `cargo test` SHALL pass
- **AND** `cargo clippy` SHALL be clean