use schemra_syntax::{Diagnostic, Severity, SourceLocation, SourceSpan};

use super::report_is_ok;

fn diagnostic(severity: Severity) -> Diagnostic {
    Diagnostic {
        code: "TEST_DIAGNOSTIC".to_owned(),
        severity,
        message: "test diagnostic".to_owned(),
        span: SourceSpan::point(SourceLocation {
            offset: 0,
            line: 1,
            column: 1,
        }),
        hint: None,
    }
}

#[test]
fn default_check_allows_warnings() {
    assert!(report_is_ok(
        "visual",
        &[diagnostic(Severity::Warning)],
        false
    ));
}

#[test]
fn strict_check_rejects_warnings() {
    assert!(!report_is_ok(
        "visual",
        &[diagnostic(Severity::Warning)],
        true
    ));
}

#[test]
fn errors_and_unknown_sources_fail_in_every_mode() {
    assert!(!report_is_ok(
        "visual",
        &[diagnostic(Severity::Error)],
        false
    ));
    assert!(!report_is_ok(
        "visual",
        &[diagnostic(Severity::Error)],
        true
    ));
    assert!(!report_is_ok("unknown", &[], false));
    assert!(!report_is_ok("unknown", &[], true));
}
