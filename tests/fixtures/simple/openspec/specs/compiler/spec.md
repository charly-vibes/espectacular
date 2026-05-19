# Capability: compiler

## ADDED Requirements

### Requirement: Input validation

#### Scenario: Empty input rejected
- **GIVEN** an empty input string
- **WHEN** the compiler processes it
- **THEN** it returns an error

#### Scenario: Null bytes rejected
- **GIVEN** input containing null bytes
- **WHEN** the compiler processes it
- **THEN** it returns an error
