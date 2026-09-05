//! One deadline for every test that waits for something to happen.
//!
//! Integration tests here poll or `tokio::time::timeout` until a condition
//! holds. The old deadlines were sized for a developer's idle machine: 3 to
//! 30 seconds against work that finishes in tens of milliseconds. CI runs on
//! a GitHub-hosted 2-core runner, and under that little CPU the same work can
//! miss a 10-second deadline while being entirely correct. Four such tests
//! went red in one day, all of them timeouts, none of them a real fault.
//!
//! Measured 2026-09-05 on `interrupt_unblocks_a_run_awaiting_input`
//! (`crates/engine/tests/e2e.rs`), which takes 0.16 s:
//!
//! | Condition                          | Result       |
//! |------------------------------------|--------------|
//! | idle, 10 runs                       | 10/10 pass  |
//! | 6 concurrent e2e suites, 10 runs    | 10/10 pass  |
//! | 32 spinners on 16 cores, 10 runs    | 9/10 pass   |
//!
//! The one failure was the CI failure, verbatim: `timed out waiting for entry
//! stamped aborted`, `e2e.rs:182`.
//!
//! [`WAIT`] is deliberately far larger than any of these tests needs. A
//! deadline is not an assertion about speed - it only stops a hung test from
//! hanging the suite forever. Nothing is lost by making it generous, and a
//! test that really hangs still fails, one minute later.
//!
//! What must NOT use it: upper-bound assertions ("this settled in under 3 s"),
//! quiet windows that assert nothing arrives, and harness configuration such
//! as handshake or grace timeouts. Those numbers mean something. Raising them
//! would change what the test proves.

use std::time::Duration;

/// How long a test waits for something that is expected to happen.
pub const WAIT: Duration = Duration::from_secs(60);
