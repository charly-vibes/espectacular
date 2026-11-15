//! Batch eligibility: which bindings may join one batched invocation.
//!
//! Purpose: admit only scenario-scoped vitest bindings whose pattern ah can
//! also re-match against structured output, so attribution never has to give
//! up on a binding that the batched run itself accepted.
//!
//! Responsibilities: recognize exactly `--testNamePattern=<regex>` flags and
//! reject patterns carrying regex constructs the Rust engine cannot compile.
//!
//! Rationale: a false positive costs only a per-binding spawn, a false
//! negative would break attribution (design eligibility guard).

use crate::contracts::TestEntry;

/// A binding eligible for batching: a vitest binding whose flags are exactly
/// a scenario-scoped test-name pattern.
#[derive(Debug, Clone)]
pub(crate) struct BatchableBinding {
    pub pattern: String,
    pub entry: TestEntry,
}

/// A vitest binding is batchable when its flags are exactly
/// `--testNamePattern=<regex>` and the pattern carries no JS-only regex
/// constructs (lookaround, backreferences) — ah re-matches patterns against
/// the structured output with the Rust regex engine, which cannot compile
/// those, so such bindings keep per-binding spawns (batch-runner-spawns
/// task 1.4; design eligibility guard).
pub(crate) fn batchable_pattern(entry: &TestEntry) -> Option<String> {
    let flags = entry.flags.as_deref()?;
    let pattern = flags.strip_prefix("--testNamePattern=")?;
    if pattern.is_empty() || has_js_only_constructs(pattern) {
        None
    } else {
        Some(pattern.to_string())
    }
}

/// Detect regex constructs the Rust regex engine cannot compile: lookaround
/// (`(?=` `(?!` `(?<=` `(?<!`) and backreferences (`\1`–`\9`, `\k<name>`).
/// Escape-aware: `\(?=` is a literal paren, not a lookahead; `\n` is a
/// newline escape, not a backreference; `(?<name>` is a named group, not a
/// lookbehind. Conservative in the safe direction — a false positive costs
/// only a per-binding spawn, a false negative would break attribution.
fn has_js_only_constructs(pattern: &str) -> bool {
    let bytes = pattern.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => {
                if i + 1 < bytes.len() {
                    let next = bytes[i + 1];
                    // Backreference: \1–\9 (not \0, the NUL escape) and
                    // named backreference \k.
                    if (next.is_ascii_digit() && next != b'0') || next == b'k' {
                        return true;
                    }
                }
                i += 2; // skip the escaped character
            }
            b'(' if i + 2 < bytes.len() && bytes[i + 1] == b'?' => {
                let lookaround = match bytes[i + 2] {
                    b'=' | b'!' => true, // (?= / (?! lookaheads
                    b'<' => i + 3 < bytes.len() && (bytes[i + 3] == b'=' || bytes[i + 3] == b'!'), // (?<= / (?<! lookbehinds; (?<name> is a named group
                    _ => false, // (?P<name>, (?:, (?flags — dual-engine
                };
                if lookaround {
                    return true;
                }
                i += 1;
            }
            _ => i += 1,
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::batchable_pattern;
    use crate::contracts::TestEntry;

    // Eligibility: only exact `--testNamePattern=<regex>` flags batch.
    #[test]
    fn batchable_pattern_recognizes_only_pattern_scoped_flags() {
        assert_eq!(
            batchable_pattern(&TestEntry {
                flags: Some("--testNamePattern=p_a".to_string()),
                command: None,
                timeout_seconds: None,
            })
            .unwrap(),
            "p_a"
        );
        assert!(batchable_pattern(&TestEntry {
            flags: Some("src/ui.test.ts".to_string()),
            command: None,
            timeout_seconds: None,
        })
        .is_none());
        assert!(batchable_pattern(&TestEntry {
            flags: Some("--testNamePattern=".to_string()),
            command: None,
            timeout_seconds: None,
        })
        .is_none());
        assert!(batchable_pattern(&TestEntry {
            flags: None,
            command: None,
            timeout_seconds: None,
        })
        .is_none());
    }

    // batch-runner-spawns task 1.4: JS-only regex constructs (lookaround,
    // backreferences) cannot compile under the Rust regex engine ah uses for
    // attribution — such bindings are excluded from batching at eligibility
    // time and keep per-binding spawns instead of paying a batched run plus
    // an attribution fallback re-run.
    #[test]
    fn batchable_pattern_rejects_js_only_regex_constructs() {
        let js_only = [
            "p_(?=b)",     // lookahead
            "p_(?!b)",     // negative lookahead
            "(?<=b)p",     // lookbehind
            "(?<!b)p",     // negative lookbehind
            "(p_a)\\1",    // backreference
            "(p_a)\\k<g>", // named backreference
            "p_\\9",       // high backreference
        ];
        for pattern in js_only {
            assert!(
                batchable_pattern(&TestEntry {
                    flags: Some(format!("--testNamePattern={pattern}")),
                    command: None,
                    timeout_seconds: None,
                })
                .is_none(),
                "JS-only pattern must not batch: {pattern}"
            );
        }
    }

    // Constructs that LOOK JS-only but are legal in both engines (or escaped
    // literals) must stay batchable — the guard is conservative but not
    // overbroad.
    #[test]
    fn batchable_pattern_keeps_dual_engine_constructs() {
        let batchable = [
            "p_a|p_b",       // alternation
            "p_\\d+",        // digit class
            "p_\\n",         // escaped letter (not a backref)
            "p_\\0",         // NUL escape, not backreference \0
            "(p_a)(p_b)",    // capturing groups
            "(?P<name>p_a)", // Rust named group
            "(?<name>p_a)",  // named group (not lookbehind)
            "\\(\\?=",       // escaped parens + equals: literal text
        ];
        for pattern in batchable {
            assert!(
                batchable_pattern(&TestEntry {
                    flags: Some(format!("--testNamePattern={pattern}")),
                    command: None,
                    timeout_seconds: None,
                })
                .is_some(),
                "dual-engine pattern must stay batchable: {pattern}"
            );
        }
    }
}
