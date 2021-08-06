use crate::tester::RaftTester;
use log::*;
use madsim::{time, Runtime};
use std::time::Duration;

/// The tester generously allows solutions to complete elections in one second
/// (much more than the paper's range of timeouts).
const RAFT_ELECTION_TIMEOUT: Duration = Duration::from_millis(1000);

// TODO: #[madsim::test]
#[test]
fn initial_election_2a() {
    let rt = Runtime::new();
    rt.block_on(async_initial_election_2a());
}

async fn async_initial_election_2a() {
    let servers = 3;
    let mut tester = RaftTester::new(servers).await;

    info!("Test (2A): initial election");

    // is a leader elected?
    tester.check_one_leader().await;

    // sleep a bit to avoid racing with followers learning of the
    // election, then check that all peers agree on the term.
    time::sleep(Duration::from_millis(50)).await;
    let term1 = tester.check_terms();

    // does the leader+term stay the same if there is no network failure?
    time::sleep(2 * RAFT_ELECTION_TIMEOUT).await;
    let term2 = tester.check_terms();
    if term1 != term2 {
        warn!("warning: term changed even though there were no failures")
    }

    // there should still be a leader.
    tester.check_one_leader().await;

    tester.end();
}
