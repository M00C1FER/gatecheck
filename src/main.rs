//! gatecheck binary — scans stdin or a file for secrets/license issues.
//!
//! Exit codes:
//!   0  — no findings at or above fail threshold
//!   1  — findings at or above fail threshold (commit / CI should abort)
//!   2  — usage / config error

use gatecheck::{scan, should_fail, Config, Finding};
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::Path;
use std::process::{Command, ExitCode};

fn print_help() {
    eprintln!(
        r#"gatecheck — ultra-fast pre-commit secret + license scanner

Usage:
  gatecheck [--config PATH] [--staged] [--diff] [--list-rules]
            [--threshold SEVERITY] [--quiet] [--json] [FILE...]

Modes (mutually exclusive):
  (default)        scan FILEs (or stdin if no FILEs given)
  --staged         scan `git diff --cached` (use as a pre-commit hook)
  --diff           scan `git diff HEAD` (uncommitted working-tree changes)
  --list-rules     print the built-in rule names

Options:
  --config PATH    custom rules config (default: ./gatecheck.toml if present)
  --threshold S    fail at SEVERITY: critical|high|medium|low (default: high)
  --quiet          suppress output, only set exit code
  --json           emit findings as JSON

Examples:
  gatecheck README.md
  echo "AKIAIOSFODNN7EXAMPLE" | gatecheck
  gatecheck --staged                # use as pre-commit hook
"#
    );
}

#[derive(Default)]
struct Args {
    config_path: Option<String>,
    files: Vec<String>,
    staged: bool,
    diff: bool,
    list_rules: bool,
    threshold: Option<String>,
    quiet: bool,
    json: bool,
    help: bool,
}

fn parse_args() -> Args {
    let mut a = Args::default();
    let mut iter = env::args().skip(1).peekable();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-h" | "--help" => a.help = true,
            "--staged" => a.staged = true,
            "--diff" => a.diff = true,
            "--list-rules" => a.list_rules = true,
            "--quiet" => a.quiet = true,
            "--json" => a.json = true,
            "--config" => {
                a.config_path = iter.next();
            }
            "--threshold" => {
                a.threshold = iter.next();
            }
            other => a.files.push(other.to_string()),
        }
    }
    a
}

fn load_config(path: Option<&str>) -> Config {
    let candidate = path
        .map(String::from)
        .or_else(|| Some("gatecheck.toml".to_string()));
    if let Some(p) = candidate {
        if Path::new(&p).is_file() {
            if let Ok(s) = fs::read_to_string(&p) {
                if let Ok(cfg) = toml::from_str::<Config>(&s) {
                    return cfg;
                }
            }
        }
    }
    Config::default()
}

fn collect_input(args: &Args) -> io::Result<String> {
    if args.staged {
        let out = Command::new("git").args(["diff", "--cached"]).output()?;
        if !out.status.success() {
            eprintln!("[gatecheck] `git diff --cached` failed");
            return Ok(String::new());
        }
        return Ok(String::from_utf8_lossy(&out.stdout).into_owned());
    }
    if args.diff {
        let out = Command::new("git").args(["diff", "HEAD"]).output()?;
        return Ok(String::from_utf8_lossy(&out.stdout).into_owned());
    }
    if !args.files.is_empty() {
        let mut combined = String::new();
        for path in &args.files {
            let body = fs::read_to_string(path)
                .map_err(|e| io::Error::new(e.kind(), format!("{}: {}", path, e)))?;
            combined.push_str(&body);
            combined.push('\n');
        }
        return Ok(combined);
    }
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)?;
    Ok(buf)
}

fn print_findings_human(findings: &[Finding]) {
    if findings.is_empty() {
        eprintln!("[gatecheck] no findings");
        return;
    }
    for f in findings {
        eprintln!(
            "[{}] line {}: {} — {}\n  matched: {:?}",
            f.severity.as_str(),
            f.line,
            f.rule,
            f.explanation,
            f.matched
        );
    }
    eprintln!("[gatecheck] {} finding(s)", findings.len());
}

fn print_findings_json(findings: &[Finding]) {
    print!("[");
    for (i, f) in findings.iter().enumerate() {
        if i > 0 {
            print!(",");
        }
        print!(
            "{{\"rule\":{:?},\"severity\":\"{}\",\"line\":{},\"matched\":{:?},\"explanation\":{:?}}}",
            f.rule,
            f.severity.as_str(),
            f.line,
            f.matched,
            f.explanation
        );
    }
    println!("]");
}

fn main() -> ExitCode {
    let args = parse_args();
    if args.help {
        print_help();
        return ExitCode::SUCCESS;
    }
    if args.list_rules {
        for r in gatecheck::rule_names() {
            println!("{}", r);
        }
        return ExitCode::SUCCESS;
    }

    let mut cfg = load_config(args.config_path.as_deref());
    if let Some(ref t) = args.threshold {
        cfg.fail_at = t.clone();
    }
    let threshold = cfg.fail_threshold();

    let input = match collect_input(&args) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[gatecheck] {}", e);
            return ExitCode::from(2);
        }
    };

    let findings = scan(&input, &cfg);
    if !args.quiet {
        if args.json {
            print_findings_json(&findings);
        } else {
            print_findings_human(&findings);
        }
    }

    let fail = should_fail(&findings, threshold);
    if fail {
        ExitCode::from(1)
    } else {
        ExitCode::from(0)
    }
}

#[cfg(test)]
mod main_tests {
    use super::*;
    use gatecheck::{Finding, Severity};

    #[test]
    fn fail_threshold_logic() {
        let critical = vec![Finding {
            rule: "x",
            severity: Severity::Critical,
            line: 1,
            matched: "x".to_string(),
            explanation: "x",
        }];
        assert!(should_fail(&critical, Severity::High));
        assert!(should_fail(&critical, Severity::Critical));

        let medium = vec![Finding {
            rule: "x",
            severity: Severity::Medium,
            line: 1,
            matched: "x".to_string(),
            explanation: "x",
        }];
        assert!(!should_fail(&medium, Severity::High));
        assert!(should_fail(&medium, Severity::Medium));
    }
}
