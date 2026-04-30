use gatecheck::{rule_names, scan, should_fail, Config, Severity};

#[test]
fn detects_aws_access_key() {
    let f = scan("token = AKIAIOSFODNN7EXAMPLE", &Config::default());
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].rule, "aws-access-key");
    assert_eq!(f[0].severity, Severity::Critical);
}

#[test]
fn detects_github_token() {
    let f = scan("export GH_TOKEN=ghp_aBcDeFgHiJkLmNoPqRsTuVwXyZ1234567890", &Config::default());
    assert!(f.iter().any(|x| x.rule == "github-token"));
}

#[test]
fn detects_private_key_block() {
    let f = scan("-----BEGIN RSA PRIVATE KEY-----\nABC\n-----END RSA PRIVATE KEY-----", &Config::default());
    assert!(f.iter().any(|x| x.rule == "private-key"));
}

#[test]
fn detects_jwt() {
    let jwt = "Bearer eyJhbGciOi.eyJzdWIiOi.aBcDeFgHiJ";
    let f = scan(jwt, &Config::default());
    assert!(f.iter().any(|x| x.rule == "jwt" || x.rule == "bearer-token"));
}

#[test]
fn no_finding_in_clean_text() {
    let f = scan("just a normal commit message about fixing a bug", &Config::default());
    assert!(f.is_empty(), "unexpected: {:?}", f);
}

#[test]
fn exemption_pattern_skips_line() {
    let cfg = Config {
        exempt_patterns: vec!["EXAMPLE_KEY".to_string()],
        ..Default::default()
    };
    let f = scan("# EXAMPLE_KEY: AKIAIOSFODNN7EXAMPLE", &cfg);
    assert!(f.is_empty(), "exemption should skip; got {:?}", f);
}

#[test]
fn disable_rule_drops_finding() {
    let cfg = Config {
        disable: vec!["aws-access-key".to_string()],
        ..Default::default()
    };
    let f = scan("AKIAIOSFODNN7EXAMPLE", &cfg);
    assert!(f.is_empty());
}

#[test]
fn fail_threshold_critical_only() {
    let f = scan("Bearer abc123def456ghi789jkl0mnopq", &Config::default());
    assert!(!f.is_empty());
    // 'medium' severity finding should NOT trigger fail_at='critical'
    assert!(!should_fail(&f, Severity::Critical));
    // ...but should trigger at 'medium'
    assert!(should_fail(&f, Severity::Medium));
}

#[test]
fn rule_names_non_empty() {
    let names = rule_names();
    assert!(names.len() >= 9, "expected ≥9 built-in rules");
    assert!(names.contains(&"aws-access-key"));
    assert!(names.contains(&"private-key"));
}

#[test]
fn line_numbers_are_1_indexed() {
    let body = "hello\nworld\nAKIAIOSFODNN7EXAMPLE\nbye";
    let f = scan(body, &Config::default());
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].line, 3);
}
