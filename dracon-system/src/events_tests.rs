//! Tests for events.rs (event types and severity classification)
//!
//! These tests verify the event components after extraction from main.rs.

#[test]
fn event_severity_debug() {
    use crate::EventSeverity;
    let sev = EventSeverity::Warn;
    let debug = format!("{:?}", sev);
    assert!(debug.contains("Warn"));
}

#[test]
fn dracon_event_debug() {
    use crate::{DraconEvent, EventSeverity};
    let event = DraconEvent {
        domain: "guard".to_string(),
        severity: EventSeverity::Warn,
        path: Some("/tmp/test".to_string()),
        message: "test warning".to_string(),
        timestamp: 1234567890,
    };
    let debug = format!("{:?}", event);
    assert!(debug.contains("guard"));
    assert!(debug.contains("Warn"));
    assert!(debug.contains("test warning"));
}

#[test]
fn dracon_event_all_severity_levels() {
    use crate::EventSeverity;
    // Just verify all variants exist
    let _ = EventSeverity::Debug;
    let _ = EventSeverity::Info;
    let _ = EventSeverity::Warn;
    let _ = EventSeverity::Error;
    let _ = EventSeverity::Critical;
}

#[test]
fn dracon_event_with_null_path() {
    use crate::{DraconEvent, EventSeverity};
    let event = DraconEvent {
        domain: "system".to_string(),
        severity: EventSeverity::Info,
        path: None,
        message: "no path event".to_string(),
        timestamp: 0,
    };
    assert!(event.path.is_none());
    assert_eq!(event.message, "no path event");
}

#[test]
fn dracon_event_timestamp_persists() {
    use crate::{DraconEvent, EventSeverity};
    let ts: u64 = 9876543210;
    let event = DraconEvent {
        domain: "test".to_string(),
        severity: EventSeverity::Critical,
        path: None,
        message: "critical event".to_string(),
        timestamp: ts,
    };
    assert_eq!(event.timestamp, ts);
}

#[test]
fn emit_event_does_not_panic() {
    use crate::{DraconEvent, EventSeverity};
    let event = DraconEvent {
        domain: "test".to_string(),
        severity: EventSeverity::Info,
        path: None,
        message: "test".to_string(),
        timestamp: 0,
    };
    // Should not panic — just records the event
    crate::emit_event(&event);
}