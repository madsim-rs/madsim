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
    use std::time::Duration;

    #[test]
    fn ctrl_c() {
        let runtime = Runtime::new();
        let node1 = runtime.create_node().build();

        runtime.block_on(async move {
            for _ in 0..2 {
                let h = node1.spawn(async move {
                    crate::signal::ctrl_c().await.unwrap();
                });
                time::sleep(Duration::from_secs(1)).await;
                assert!(!h.is_finished());

                Handle::current().send_ctrl_c(node1.id());

                time::sleep(Duration::from_secs(1)).await;
                assert!(h.is_finished());
            }
        });
    }
}
