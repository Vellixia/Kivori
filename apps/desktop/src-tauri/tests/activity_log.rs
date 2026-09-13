//! Native activity-log contract: chronological in-memory history and safe DTO projection.

use std::sync::{Arc, Barrier};

use kivori_desktop::activity::{ActivityEventKind, ActivityLog, ActivityMetadata};
use kivori_desktop::ipc::dto;
use kivori_desktop::ipc::events;
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
    assert_eq!(value["severity"], "info");
    assert_eq!(value["source"], "connection");
    assert_eq!(value["outcome"], "observed");
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
fn activity_log_clamps_oversized_capacity_to_256_entries() {
    let log = ActivityLog::new(300);
    for _ in 0..300 {
        log.record(ActivityEventKind::ConnectionStateChanged, None);
    }

    let recent = log.recent(300);
    assert_eq!(recent.len(), 256);
    assert!(recent.windows(2).all(|pair| pair[0].id() < pair[1].id()));
}

#[test]
fn concurrent_records_are_returned_in_strictly_increasing_id_order() {
    for _ in 0..32 {
        let log = Arc::new(ActivityLog::new(128));
        let start = Arc::new(Barrier::new(33));
        let mut workers = Vec::new();
        for _ in 0..32 {
            let log = Arc::clone(&log);
            let start = Arc::clone(&start);
            workers.push(std::thread::spawn(move || {
                start.wait();
                log.record(ActivityEventKind::ConnectionStateChanged, None);
            }));
        }
        start.wait();
        for worker in workers {
            worker.join().expect("activity writer completes");
        }

        let recent = log.recent(128);
        assert_eq!(recent.len(), 32);
        assert!(recent.windows(2).all(|pair| pair[0].id() < pair[1].id()));
    }
}

#[test]
fn activity_taxonomy_serializes_firmware_failure_with_closed_classification() {
    let log = ActivityLog::new(1);
    let event = log.record(ActivityEventKind::FirmwareUpdateFailed, None);

    let value = serde_json::to_value(dto::activity_event(&event)).expect("activity DTO serializes");
    assert_eq!(value["type"], "firmwareUpdateFailed");
    assert_eq!(value["severity"], "error");
    assert_eq!(value["source"], "firmware");
    assert_eq!(value["outcome"], "failed");
}

#[test]
fn record_to_emission_seam_uses_the_live_activity_contract() {
    let event = ActivityLog::new(1).record(ActivityEventKind::ConnectionStateChanged, None);
    let emission = events::activity_log_emission(&event);

    assert_eq!(emission.name, "activity-log://event");
    assert_eq!(
        emission.payload.event_type,
        dto::ActivityEventTypeDto::ConnectionStateChanged
    );
    assert_eq!(emission.payload.severity, dto::ActivitySeverityDto::Info);
    assert_eq!(emission.payload.source, dto::ActivitySourceDto::Connection);
    assert_eq!(emission.payload.outcome, dto::ActivityOutcomeDto::Observed);
}
