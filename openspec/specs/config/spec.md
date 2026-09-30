---
id: spec
kind: intent
statement: "WHEN the tool adopts genesis::config THE tool SHALL implement ConfigFile, delegate all config file I/O to genesis, register its config with ConfigRegistry at startup, and remove dead config code without new clippy warnings."
---

# config spec delta: adopt genesis::config

## Constraints

| id | kind | expr | traces_to |
|----|------|------|-----------|
| C-configfile-impl | invariant | the tool's config struct implements `genesis::config::ConfigFile` and all config file I/O (read, write, parse) delegates to genesis, with `cargo test` passing | [[spec]] |
| C-startup-registration | invariant | at startup the tool registers its config struct with `ConfigRegistry` | [[spec]] |
| C-configstore-advisory | advisory | config discovery and validation SHOULD use `ConfigStore` | [[spec]] |
| C-dead-code-removed | invariant | after adoption the old config parsing code is removed, `cargo clippy` introduces no new warnings, and `cargo test` passes | [[spec]] |

## Model

### States

- `own-config`
- `genesis-config`
- `registered`

### Transitions

| id | from | to | guard |
|----|------|----|-------|
| t-adopt | own-config | genesis-config | [[spec.C-configfile-impl]] |
| t-register | genesis-config | registered | [[spec.C-startup-registration]] |
| t-prune | registered | registered | [[spec.C-dead-code-removed]] |
| t-store | registered | registered | [[spec.C-configstore-advisory]] |

## Properties

| id | kind | derives_from | generator | predicate |
|----|------|--------------|-----------|-----------|
| P-configfile | unit | [[spec.C-configfile-impl]] | the adopted config struct | it implements `ConfigFile`; every read, write, and parse call routes through genesis; tests pass |
| P-registration | unit | [[spec.C-startup-registration]] | a startup run | the config struct is registered with `ConfigRegistry` before config is first read |
| P-configstore | unit | [[spec.C-configstore-advisory]] | config discovery and validation paths | they use `ConfigStore` where the advisory is honored |
| P-prune | unit | [[spec.C-dead-code-removed]] | the post-adoption tree | no old config parsing code remains; `cargo clippy` adds no new warnings and `cargo test` passes |

## Purpose

Adopt `genesis::config` as the shared config substrate: the tool's config
implements `ConfigFile`, all file I/O delegates to genesis, startup registers
with `ConfigRegistry`, and dead local config code is removed cleanly.

## Requirements
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

