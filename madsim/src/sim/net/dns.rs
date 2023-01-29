use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};

/// The global DNS server in the cluster.
#[derive(Debug)]
pub struct DnsServer {
    records: HashMap<String, IpAddr>,
}

impl Default for DnsServer {
    fn default() -> Self {
        let mut records = HashMap::new();
        records.insert("localhost".into(), Ipv4Addr::LOCALHOST.into());
        Self { records }
    }
}

impl DnsServer {
    pub fn add(&mut self, name: &str, ip: IpAddr) {
        self.records.insert(name.to_string(), ip);
    }

    pub fn lookup(&self, name: &str) -> Option<IpAddr> {
        self.records.get(name).cloned()
    }
}
