## MODIFIED Requirements

### Requirement: Change Overlay Scope
The system SHALL support checking selected OpenSpec changes as overlays on deployed specs.

#### Scenario: Apply modified scenario text from change overlay
- **GIVEN** a change delta defines scenario `old-behavior` for spec `compiler`, which exists in the deployed specs, with different body text
- **AND** the change stages a contract at `.espectacular/changes/<change>/compiler/old-behavior.toml`
- **WHEN** a user runs `ah check --changes <change>`
- **THEN** the overlay scenario text replaces the deployed scenario text in scope
- **AND** no `overlay-conflict` finding is emitted

#### Scenario: Reject unsignaled scenario redefinition
- **GIVEN** a change delta defines scenario `old-behavior` for spec `compiler`, which exists in the deployed specs, with different body text
- **AND** the change stages no contract for `old-behavior`
- **WHEN** a user runs `ah check --changes <change>`
- **THEN** the command emits an `overlay-conflict` structural finding
- **AND** exits non-zero
