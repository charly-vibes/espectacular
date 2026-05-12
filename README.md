# espectacular

[![tracked with wai](https://img.shields.io/badge/tracked%20with-wai-blue)](https://github.com/charly-vibes/wai)

Behavioral verification layer for the charly AI development ecosystem.

## Tooling

This repo is configured to use:

- `wai` — project context, reasoning, handoffs
- `bd` (beads) — issue tracking and dependencies
- `openspec` — specs and change proposals
- `dont` — epistemic claim tracking and evidence grounding

## Quick start

```bash
just prime
just status
just validate
```

Or run the tools directly:

```bash
wai status
bd ready
openspec list
dont prime --plain
```

## Notes

- `wai` state lives in `.wai/`
- beads state lives in `.beads/`
- OpenSpec files live in `openspec/`
- `dont` state lives in `.dont/`
