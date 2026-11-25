# Oracle Scripts

Oracle scripts are user-defined validators run during pipeline gate checks.

## Convention

- Place scripts here: `.wai/resources/oracles/<name>[.sh|.py]`
- Scripts must be executable (`chmod +x`)
- Exit 0 = pass, non-zero = fail
- Write failure reasons to stderr
- Default scope: one invocation per artifact (`<script> <artifact-path>`)
- Cross-artifact scope: `scope = "all"` passes all paths at once

## Example

```bash
#!/usr/bin/env bash
# example-check.sh — verify artifact contains required sections
grep -q '## Constraints' "$1" || { echo 'Missing ## Constraints section' >&2; exit 1; }
```

Configure in your pipeline TOML:
```toml
[[steps.gate.oracles]]
name = "example-check"
```
