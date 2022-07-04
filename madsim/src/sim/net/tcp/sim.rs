use std::time::Duration;

use log::debug;

use super::network::TcpNetwork;
use crate::{
    plugin,
    rand::{GlobalRng, Rng},
    task::NodeId,
    time::TimeHandle,
};

/// a Tcp simulator
///
/// simulate the delay of packet sending, and timout resend.
#[cfg_attr(docsrs, doc(cfg(madsim)))]
pub struct TcpSim {
    rand: GlobalRng,
    time: TimeHandle,
    network: TcpNetwork,
}

impl plugin::Simulator for TcpSim {
    fn new(
        rand: &crate::rand::GlobalRng,
        time: &crate::time::TimeHandle,
        config: &crate::Config,
    ) -> Self
    where
        Self: Sized,
    {
        Self {
            rand: rand.clone(),
            time: time.clone(),
            network: TcpNetwork::new(rand.clone(), time.clone(), config.clone()),
        }
    }
}

impl TcpSim {
    /// Update tcp configurations.
    pub fn update_config(&self, f: impl FnOnce(&mut crate::Config)) {
        self.network.update_config(f);
    }

    /// Reset a node.
    ///
    /// All connections will be closed.
    pub fn reset_node(&self, node: NodeId) {
        match self.network.reset_node(node) {
            Ok(()) => (),
            Err(e) => debug!("{}: {}", e, node),
        }
    }

    /// Connect a node to the tcp network.
    pub fn connect(&self, id: NodeId) {
        self.network.unclog_node(id);
    }

    /// Disconnect a node from the tcp network.
    pub fn disconnect(&self, id: NodeId) {
        self.network.clog_node(id);
    }

    /// Connect a pair of nodes.
    pub fn connect2(&self, node1: NodeId, node2: NodeId) {
        self.network.unclog_link(node1, node2);
        self.network.unclog_link(node2, node1);
    }

    /// Disconnect a pair of nodes.
    pub fn disconnect2(&self, node1: NodeId, node2: NodeId) {
        self.network.clog_link(node1, node2);
        self.network.clog_link(node2, node1);
    }

    pub(crate) async fn rand_delay(&self) {
        let delay = Duration::from_micros(self.rand.with(|rng| rng.gen_range(0..5)));
        self.time.sleep(delay).await;
    }

    pub(crate) fn network(&self) -> &TcpNetwork {
        &self.network
    }
}
