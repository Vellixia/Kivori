//! Deadline policy tests for the native device task.

use std::time::Duration;

use kivori_desktop::runtime::device_task::{
    ConnectionDeadlines, HANDSHAKE_TIMEOUT, HEARTBEAT_INTERVAL,
};

#[test]
fn connecting_link_times_out_and_clears_its_deadline() {
    let mut deadlines = ConnectionDeadlines::new();
    let opened_at = Duration::from_secs(10);

    deadlines.on_port_opened(opened_at);
    assert!(
        !deadlines.handshake_timed_out(opened_at + HANDSHAKE_TIMEOUT - Duration::from_millis(1))
    );
    assert!(deadlines.handshake_timed_out(opened_at + HANDSHAKE_TIMEOUT));

    deadlines.on_link_lost();
    assert!(!deadlines.handshake_timed_out(opened_at + HANDSHAKE_TIMEOUT));
}

#[test]
fn connected_link_waits_one_interval_between_heartbeats() {
    let mut deadlines = ConnectionDeadlines::new();
    let connected_at = Duration::from_secs(20);

    deadlines.on_port_opened(connected_at - Duration::from_secs(1));
    deadlines.on_handshake_complete(connected_at);
    assert!(!deadlines.heartbeat_due(connected_at + HEARTBEAT_INTERVAL - Duration::from_millis(1)));
    assert!(deadlines.heartbeat_due(connected_at + HEARTBEAT_INTERVAL));

    deadlines.on_heartbeat_sent(connected_at + HEARTBEAT_INTERVAL);
    assert!(!deadlines.heartbeat_due(connected_at + HEARTBEAT_INTERVAL));
    assert!(deadlines.heartbeat_due(connected_at + HEARTBEAT_INTERVAL * 2));
}
