use schemra_ir::{
    NodeId, SourceLocation, SourceSpan, Timeline, TimelineEvent, TimelineEventKind, TokenId,
};

#[test]
fn timeline_events_round_trip_with_source_spans_and_typed_endpoints() {
    let source_span = SourceSpan::point(SourceLocation {
        offset: 4,
        line: 2,
        column: 3,
    });
    let timeline = Timeline {
        duration_ms: 250,
        events: vec![TimelineEvent {
            start_ms: 0,
            end_ms: 250,
            kind: TimelineEventKind::Move {
                token: TokenId("request".into()),
                from: NodeId("client".into()),
                to: NodeId("proxy".into()),
            },
            span: source_span,
        }],
    };

    let encoded = serde_json::to_value(&timeline).unwrap();
    let decoded: Timeline = serde_json::from_value(encoded).unwrap();
    assert_eq!(decoded, timeline);
}
