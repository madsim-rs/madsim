//! IP Virtual Server.

use super::IpProtocol;
use spin::Mutex;
use std::collections::HashMap;
use std::net::SocketAddr;

/// IP Virtual Server.
#[derive(Default, Debug)]
pub struct IpVirtualServer {
    services: Mutex<HashMap<ServiceAddr, Service>>,
}

/// Virtual service address.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum ServiceAddr {
    /// TCP
    Tcp(String),
    /// UDP
    Udp(String),
}

impl ServiceAddr {
    pub(crate) fn from_addr_proto(addr: SocketAddr, proto: IpProtocol) -> Self {
        match proto {
            IpProtocol::Tcp => Self::Tcp(addr.to_string()),
            IpProtocol::Udp => Self::Udp(addr.to_string()),
        }
    }
}

/// Virtual Service.
#[derive(Debug)]
struct Service {
    scheduler: Scheduler,
    servers: Vec<String>,
    rr_index: usize,
}

/// Algorithm for allocating TCP connections and UDP datagrams to real servers.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Scheduler {
    /// distributes jobs equally amongst the available real servers.
    RoundRobin,
}

impl IpVirtualServer {
    /// Add a virtual service.
    pub fn add_service(&self, service_addr: ServiceAddr, scheduler: Scheduler) {
        self.services.lock().insert(
            service_addr,
            Service {
                scheduler,
                servers: Vec::new(),
                rr_index: 0,
            },
        );
    }

    /// Delete a virtual service, along with any associated real servers.
    pub fn del_service(&self, service_addr: ServiceAddr) {
        self.services.lock().remove(&service_addr);
    }

    /// Add a real server to a virtual service.
    pub fn add_server(&self, service_addr: ServiceAddr, server_addr: &str) {
        self.services
            .lock()
            .get_mut(&service_addr)
            .expect("service not found")
            .servers
            .push(server_addr.into());
    }

    /// Remove a real server from a virtual service.
    pub fn del_server(&self, service_addr: ServiceAddr, server_addr: &str) {
        self.services
            .lock()
            .get_mut(&service_addr)
            .expect("service not found")
            .servers
            .retain(|addr| addr != server_addr);
    }

    /// Get a real server from a virtual service.
    pub fn get_server(&self, service_addr: ServiceAddr) -> Option<String> {
        let mut guard = self.services.lock();
        let service = guard.get_mut(&service_addr)?;
        match service.scheduler {
            Scheduler::RoundRobin => {
                if service.servers.is_empty() {
                    return None;
                }
                let i = &mut service.rr_index;
                if *i >= service.servers.len() {
                    *i = 0;
                }
                let server = service.servers[*i].clone();
                *i += 1;
                Some(server)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_robin() {
        let ipvs = IpVirtualServer::default();
        let service = ServiceAddr::Tcp("10.0.0.1:80".into());
        ipvs.add_service(service.clone(), Scheduler::RoundRobin);
        assert_eq!(ipvs.get_server(service.clone()), None);

        ipvs.add_server(service.clone(), "192.168.1.1:80");
        ipvs.add_server(service.clone(), "192.168.1.2:80");
        ipvs.add_server(service.clone(), "192.168.1.3:80");

        assert_eq!(ipvs.get_server(service.clone()).unwrap(), "192.168.1.1:80");
        assert_eq!(ipvs.get_server(service.clone()).unwrap(), "192.168.1.2:80");
        assert_eq!(ipvs.get_server(service.clone()).unwrap(), "192.168.1.3:80");

        ipvs.del_server(service.clone(), "192.168.1.1:80");

        assert_eq!(ipvs.get_server(service).unwrap(), "192.168.1.2:80");
    }
}
