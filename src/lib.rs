//! gatecheck — ultra-fast secret + license scanner.
//!
//! Library entry points:
//!
//! ```no_run
//! use gatecheck::{scan, Config};
//! let cfg = Config::default();
//! let findings = scan("AWS key: AKIAIOSFODNN7EXAMPLE", &cfg);
//! assert!(!findings.is_empty());
//! ```

use regex::Regex;
use serde::Deserialize;
use std::sync::OnceLock;

/// Severity levels emitted by scanners.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Critical => "critical",
            Severity::High => "high",
            Severity::Medium => "medium",
            Severity::Low => "low",
        }
    }
}

/// One detection from a scanner.
#[derive(Debug, Clone)]
pub struct Finding {
    pub rule: &'static str,
    pub severity: Severity,
    pub line: usize,
    pub matched: String,
    pub explanation: &'static str,
}

/// Per-rule on/off + exemptions, loaded from gatecheck.toml.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub disable: Vec<String>,
    #[serde(default)]
    pub exempt_patterns: Vec<String>,
    #[serde(default = "default_severity")]
    pub fail_at: String,
}

fn default_severity() -> String {
    "high".to_string()
}

impl Config {
    pub fn fail_threshold(&self) -> Severity {
        match self.fail_at.to_lowercase().as_str() {
            "critical" => Severity::Critical,
            "high" => Severity::High,
            "medium" => Severity::Medium,
            "low" => Severity::Low,
            _ => Severity::High,
        }
    }

    pub fn is_disabled(&self, rule: &str) -> bool {
        self.disable.iter().any(|r| r == rule)
    }
}

/// Built-in rule set. Each rule has a name, severity, regex, and explanation.
struct Rule {
    name: &'static str,
    severity: Severity,
    regex: &'static str,
    explanation: &'static str,
}

const RULES: &[Rule] = &[
    Rule {
        name: "aws-access-key",
        severity: Severity::Critical,
        regex: r"AKIA[0-9A-Z]{16}",
        explanation: "AWS access-key ID detected — never commit; rotate immediately if leaked.",
    },
    Rule {
        name: "aws-secret",
        severity: Severity::Critical,
        regex: r#"(?i)aws.{0,20}?(?:secret|key).{0,5}[\s:=]{1,3}["']?[A-Za-z0-9/+=]{40}["']?"#,
        explanation: "Looks like an AWS secret-access-key — rotate immediately if leaked.",
    },
    Rule {
        name: "github-token",
        severity: Severity::Critical,
        regex: r"gh[pousr]_[A-Za-z0-9_]{36,}",
        explanation: "GitHub personal access token — revoke and rotate.",
    },
    Rule {
        name: "github-fine-grained",
        severity: Severity::Critical,
        regex: r"github_pat_[A-Za-z0-9_]{82}",
        explanation: "GitHub fine-grained PAT — revoke and rotate.",
    },
    Rule {
        name: "slack-token",
        severity: Severity::High,
        regex: r"xox[baprs]-[A-Za-z0-9-]{10,}",
        explanation: "Slack token — revoke at https://api.slack.com/apps.",
    },
    Rule {
        name: "jwt",
        severity: Severity::Medium,
        regex: r"eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}",
        explanation: "JWT-shaped token — verify whether it's session-bound or static.",
    },
    Rule {
        name: "private-key",
        severity: Severity::Critical,
        regex: r"-----BEGIN (?:RSA |EC |DSA |OPENSSH |)PRIVATE KEY-----",
        explanation: "Private key block — never commit private keys.",
    },
    Rule {
        name: "bearer-token",
        severity: Severity::Medium,
        regex: r"(?i)Bearer\s+[A-Za-z0-9._-]{20,}",
        explanation: "Bearer token in source — should be loaded from env at runtime.",
    },
    Rule {
        name: "generic-password",
        severity: Severity::Medium,
        regex: r#"(?i)(?:password|passwd|pwd)\s*[:=]\s*["'][^"']{8,}["']"#,
        explanation: "Hardcoded password literal — move to env var or secret manager.",
    },
    Rule {
        name: "gpl-in-mit",
        severity: Severity::High,
        regex: r"(?i)GNU GENERAL PUBLIC LICENSE",
        explanation: "GPL boilerplate detected — review compatibility with your project's license.",
    },
];

fn compiled_rules() -> &'static [(Regex, &'static Rule)] {
    static C: OnceLock<Vec<(Regex, &'static Rule)>> = OnceLock::new();
    C.get_or_init(|| {
        RULES
            .iter()
            .map(|r| (Regex::new(r.regex).expect("rule regex compiles"), r))
            .collect()
    })
}

fn compiled_exemptions(cfg: &Config) -> Vec<Regex> {
    cfg.exempt_patterns
        .iter()
        .filter_map(|p| Regex::new(p).ok())
        .collect()
}

/// Scan content (a diff hunk, a file, a string) and return findings.
pub fn scan(content: &str, cfg: &Config) -> Vec<Finding> {
    let exemptions = compiled_exemptions(cfg);
    let mut out = Vec::new();
    for (line_idx, line) in content.lines().enumerate() {
        let line_no = line_idx + 1;
        if exemptions.iter().any(|r| r.is_match(line)) {
            continue;
        }
        for (re, rule) in compiled_rules() {
            if cfg.is_disabled(rule.name) {
                continue;
            }
            if let Some(m) = re.find(line) {
                out.push(Finding {
                    rule: rule.name,
                    severity: rule.severity,
                    line: line_no,
                    matched: m.as_str().to_string(),
                    explanation: rule.explanation,
                });
            }
        }
    }
    out
}

/// True iff any finding meets or exceeds the fail-threshold.
pub fn should_fail(findings: &[Finding], threshold: Severity) -> bool {
    let order = |s: Severity| match s {
        Severity::Critical => 0,
        Severity::High => 1,
        Severity::Medium => 2,
        Severity::Low => 3,
    };
    findings.iter().any(|f| order(f.severity) <= order(threshold))
}

/// Names of all built-in rules (useful for `gatecheck --list-rules`).
pub fn rule_names() -> Vec<&'static str> {
    RULES.iter().map(|r| r.name).collect()
}
