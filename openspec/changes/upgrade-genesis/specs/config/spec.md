# config spec delta: adopt genesis::config

## ADDED Requirements

### Requirement: Shared config management

The tool SHALL adopt `genesis::config` for shared config management.

#### Scenario: config struct implements ConfigFile

- **GIVEN** the tool needs config management
- **WHEN** the tool adopts `genesis::config`
- **THEN** the tool's config SHALL implement `genesis::config::ConfigFile`
- **AND** all config file I/O (read, write, parse) SHALL delegate to genesis
- **AND** `cargo test` SHALL pass

#### Scenario: config registered at startup

- **GIVEN** the tool starts up
- **WHEN** it initializes
- **THEN** it SHALL register its config struct with `ConfigRegistry`
- **AND** it SHOULD use `ConfigStore` for config discovery and validation

#### Scenario: dead config code removed

- **GIVEN** the tool has adopted genesis::config
- **WHEN** the old config parsing code is removed
- **THEN** `cargo clippy` SHALL introduce no new warnings
- **AND** `cargo test` SHALL pass