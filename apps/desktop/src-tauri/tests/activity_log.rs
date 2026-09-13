//! Native activity-log contract: chronological in-memory history and safe DTO projection.

use kivori_desktop::activity::{ActivityEventKind, ActivityLog, ActivityMetadata};
use kivori_desktop::ipc::dto;
use kivori_desktop::ipc::events::ACTIVITY_LOG_EVENT;
use kivori_model::ConnectionState;

#[test]
fn activity_log_evicts_oldest_entries_and_returns_chronological_history() {
    let log = ActivityLog::new(3);
    for state in [
        ConnectionState::Connecting,
        ConnectionState::Connected,
        ConnectionState::Error,
        ConnectionState::Disconnected,
    ] {
        log.record(
            ActivityEventKind::ConnectionStateChanged,
            Some(ActivityMetadata::Connection {
                state,
                retry_count: 2,
                elapsed_ms: 50,
            }),
        );
    }

    let recent = log.recent(10);
    assert_eq!(recent.len(), 3);
    assert_eq!(recent[0].summary(), "Connection changed to connected.");
    assert_eq!(recent[1].summary(), "Connection changed to error.");
    assert_eq!(recent[2].summary(), "Connection changed to disconnected.");
    assert!(recent[0].id() < recent[1].id() && recent[1].id() < recent[2].id());
}

#[test]
fn activity_ids_are_unique_and_monotonically_increasing_across_logs() {
    let first_log = ActivityLog::new(1);
    let first = first_log.record(ActivityEventKind::ConnectionStateChanged, None);
    let second_log = ActivityLog::new(1);
    let second = second_log.record(ActivityEventKind::ConnectionStateChanged, None);

    assert!(second.id() > first.id());
}

#[test]
fn activity_dto_uses_closed_types_and_only_typed_optional_metadata() {
    let log = ActivityLog::new(1);
    let event = log.record(
        ActivityEventKind::ConnectionStateChanged,
        Some(ActivityMetadata::Connection {
            state: ConnectionState::Connected,
            retry_count: 4,
            elapsed_ms: 800,
        }),
    );

    let value = serde_json::to_value(dto::activity_event(&event)).expect("activity DTO serializes");
    assert_eq!(value["type"], "connectionStateChanged");
    assert_eq!(value["summary"], "Connection changed to connected.");
    assert_eq!(value["metadata"]["connection"], "connected");
    assert_eq!(value["metadata"]["retryCount"], 4);
    assert_eq!(value["metadata"]["elapsedMs"], 800);
    assert_eq!(
        value["metadata"]
            .as_object()
            .expect("typed metadata object")
            .len(),
        3,
        "metadata has an allowlisted, fixed structure"
    );
}

#[test]
fn a_new_activity_log_is_session_only_and_empty() {
    let log = ActivityLog::new(2);
    assert!(log.recent(10).is_empty());
}

#[test]
fn live_activity_event_uses_the_activity_log_contract_name() {
    assert_eq!(ACTIVITY_LOG_EVENT, "activity-log://event");
}
