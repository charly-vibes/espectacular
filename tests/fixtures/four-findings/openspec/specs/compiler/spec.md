# Capability: compiler

## ADDED Requirements

### Requirement: Input validation

#### Scenario: Has contract
- **GIVEN** a scenario with a valid contract
- **WHEN** checked
- **THEN** passes

#### Scenario: Missing contract
- **GIVEN** no sidecar TOML
- **WHEN** checked
- **THEN** missing-contract finding

#### Scenario: No tests declared
- **GIVEN** a contract with empty tests
- **WHEN** checked
- **THEN** no-tests-declared finding

#### Scenario: Duplicate id one
- **GIVEN** two headings that slugify the same
- **WHEN** checked
- **THEN** duplicate-id finding

#### Scenario: Duplicate Id One
- **GIVEN** same slug as above
- **WHEN** checked
- **THEN** duplicate-id finding
