# espectacular

Behavioral verification layer for the charly AI development ecosystem.

Compiles openspec specifications into executable assertions and detects drift between specs and code over time.

## Ecosystem context

espectacular bridges the gap between specification (openspec) and verification in the charly toolchain:

| Tool | Role |
|------|------|
| **wai** | Decision capture and reasoning workflow |
| **dont** | Epistemic grounding — claims must carry evidence |
| **pretender** | Structural quality — complexity, duplication, nesting |
| **openspec** | Specification format — requirements + scenarios |
| **espectacular** | Behavioral verification — specs become executable checks |

## Design principles

- Specs are the source of truth; assertions are derived artifacts
- Drift detection is continuous, not one-shot
- **Language-agnostic core**: Uses a JSON Schema IR and template-based emitters
- **AI-friendly Structural Frames**: Emits skeletal test stubs (archetypes) for easy implementation by AI agents
- Integrates with existing openspec workflow (proposal → implement → archive → verify)
