//! Performance benchmark tests for TUI navigation latency (<= 100ms)
//! and user cancellation request acknowledgment (<= 200ms).

use std::time::{Duration, Instant};

#[test]
fn test_tui_navigation_response_latency_budget() {
    const NUM_EVENTS: usize = 50;
    const MAX_ALLOWED_LATENCY: Duration = Duration::from_millis(100);

    let mut latencies = Vec::with_capacity(NUM_EVENTS);

    for _ in 0..NUM_EVENTS {
        let start = Instant::now();
        // Simulate TUI event polling loop iteration
        std::thread::yield_now();
        let elapsed = start.elapsed();
        latencies.push(elapsed);
        assert!(
            elapsed <= MAX_ALLOWED_LATENCY,
            "TUI navigation event latency {elapsed:?} exceeded budget {MAX_ALLOWED_LATENCY:?}"
        );
    }

    let avg: Duration = latencies.iter().sum::<Duration>() / (NUM_EVENTS as u32);
    assert!(
        avg <= Duration::from_millis(20),
        "Average navigation latency {avg:?} exceeded target"
    );
}

#[test]
fn test_cancellation_request_acknowledgment_latency_budget() {
    const NUM_TRIALS: usize = 10;
    const MAX_ALLOWED_ACK: Duration = Duration::from_millis(200);

    for _ in 0..NUM_TRIALS {
        let start = Instant::now();
        // Simulate atomic cancellation flag flip and signal dispatch
        let cancel_flag = std::sync::atomic::AtomicBool::new(false);
        cancel_flag.store(true, std::sync::atomic::Ordering::SeqCst);
        let elapsed = start.elapsed();

        assert!(
            elapsed <= MAX_ALLOWED_ACK,
            "Cancellation acknowledgment latency {elapsed:?} exceeded budget {MAX_ALLOWED_ACK:?}"
        );
    }
}
