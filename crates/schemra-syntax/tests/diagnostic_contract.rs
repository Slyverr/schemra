use schemra_syntax::{Diagnostic, Severity, SourceLocation, SourceSpan};
use serde_json::json;

fn location(offset: usize, line: usize, column: usize) -> SourceLocation {
    SourceLocation {
        offset,
        line,
        column,
    }
}

#[test]
fn point_span_uses_one_location_for_both_ends() {
    let point = location(12, 2, 5);

    assert_eq!(
        SourceSpan::point(point),
        SourceSpan {
            start: point,
            end: point
        }
    );
}

#[test]
fn severity_serializes_as_lowercase_values() {
    let values = [Severity::Error, Severity::Warning, Severity::Information]
        .map(|severity| serde_json::to_value(severity).expect("severity JSON"));

    assert_eq!(
        values,
        [json!("error"), json!("warning"), json!("information")]
    );
}

#[test]
fn error_diagnostic_serializes_its_code_message_span_and_hint() {
    let span = SourceSpan {
        start: location(4, 1, 5),
        end: location(8, 1, 9),
    };
    let diagnostic = Diagnostic::error("E_TEST", "Invalid value", span, Some("Use a valid value"));

    assert_eq!(
        serde_json::to_value(diagnostic).expect("diagnostic JSON"),
        json!({
            "code": "E_TEST",
            "severity": "error",
            "message": "Invalid value",
            "span": {
                "start": {"offset": 4, "line": 1, "column": 5},
                "end": {"offset": 8, "line": 1, "column": 9}
            },
            "hint": "Use a valid value"
        })
    );
}

#[test]
fn diagnostic_without_hint_serializes_null() {
    let diagnostic = Diagnostic::error(
        "E_TEST",
        "Invalid value",
        SourceSpan::point(location(0, 1, 1)),
        None::<String>,
    );

    assert_eq!(
        serde_json::to_value(diagnostic).expect("diagnostic JSON")["hint"],
        serde_json::Value::Null
    );
}

#[test]
fn information_constructor_marks_advisories_non_blocking() {
    let diagnostic = Diagnostic::information(
        "I_TEST",
        "This is an optional suggestion.",
        SourceSpan::point(location(0, 1, 1)),
        None::<String>,
    );

    assert_eq!(diagnostic.severity, Severity::Information);
}
