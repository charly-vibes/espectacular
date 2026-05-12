# Capability: compiler

The compiler transforms OpenSpec specifications into language-agnostic assertions and structural test frames.

## ADDED Requirements
### Requirement: Structural Frame Generation
The compiler SHALL generate structural test stubs (frames) based on archetype tags to guide implementation.

#### Scenario: Generate Rust PF Frame [PF]
- **WHEN** the compiler processes a scenario tagged with `[PF]` for a Rust target
- **THEN** it emits a Rust test function with `// [espectacular:PF]` and WHEN/THEN TODOs

#### Scenario: Generate Go SA Frame [SA]
- **WHEN** the compiler processes a scenario tagged with `[SA]` for a Go target
- **THEN** it emits a Go test function with `// [espectacular:SA]` and structured setup/action/verify phases

#### Scenario: Handle PBT Hint [PF, pbt]
- **WHEN** a scenario has both `[PF]` and `[pbt]` tags
- **THEN** the emitted frame includes a hint to use a property-based testing library
