//! Asynchronous signal handling.

/// Completes when a "ctrl-c" notification is sent to the process.
pub async fn ctrl_c() -> std::io::Result<()> {
    let mut rx = crate::context::current_task().node.ctrl_c();
    _ = rx.changed().await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        runtime::{Handle, Runtime},
        time,
    };
    use futures_util::future::pending;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    use std::time::Duration;

    #[test]
    fn ctrl_c_kill() {
        let runtime = Runtime::new();
        let node = runtime.create_node().build();

        runtime.block_on(async move {
            let h = node.spawn(async {
                pending::<()>().await;
            });
            assert!(!h.is_finished());

            // ctrl-c will kill the node
            Handle::current().send_ctrl_c(node.id());

            time::sleep(Duration::from_secs(1)).await;
            assert!(h.is_finished());
        });
    }

    #[test]
    fn ctrl_c_catch() {
        let runtime = Runtime::new();
        let node = runtime.create_node().build();

        runtime.block_on(async move {
            for _ in 0..2 {
                let flag = Arc::new(AtomicBool::new(false));
                let flag1 = flag.clone();

                let h = node.spawn(async move {
                    crate::signal::ctrl_c().await.unwrap();
                    flag1.store(true, Ordering::Relaxed);
                    pending::<()>().await;
                });
                time::sleep(Duration::from_secs(1)).await;
                assert!(!flag.load(Ordering::Relaxed));
                assert!(!h.is_finished());

                // ctrl-c will be caught and not kill the node
                Handle::current().send_ctrl_c(node.id());

                time::sleep(Duration::from_secs(1)).await;
                assert!(flag.load(Ordering::Relaxed));
                assert!(!h.is_finished());
            }
        });
    }
}
