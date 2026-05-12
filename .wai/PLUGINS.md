# Plugins

Wai auto-detects and integrates with external tools:

## Detected in this workspace:
- **git** — Version control (hooks: status, handoff)
- **beads** — Issue tracking (commands: list, show, ready)
- **openspec** — Specification management


## Custom plugins

Add YAML files to `.wai/plugins/` to register custom plugins:

```yaml
name: my-tool
description: Integration with my-tool
detector:
type: directory
path: .my-tool
commands:
- name: list
description: List my-tool items
passthrough: my-tool list
read_only: true
hooks:
on_status:
command: my-tool status
inject_as: tool_status
```

Run `wai plugin list` to see all available plugins.
