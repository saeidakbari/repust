/// This module contains the implementation of the `Meta` struct and related functions.
/// The `Meta` struct represents metadata about the cluster, including the cluster name, port, and IP address.
/// It also provides functions to retrieve the IP address, port, and cluster name.
/// The module also includes a thread-local storage for storing the `Meta` instance.
///
/// # Example
///
/// ```
/// use crate::com::meta::{Meta, load_meta, get_ip, get_port, get_cluster};
/// use crate::com::config::ClusterConfig;
///
/// let cc = ClusterConfig {
///     name: "my_cluster".to_string(),
///     listen_addr: "127.0.0.1:8080".to_string(),
/// };
///
/// let meta = load_meta(cc, None);
///
/// meta_init(meta);
///
/// let ip = get_ip();
/// let port = get_port();
/// let cluster = get_cluster();
/// println!("IP: {}", ip);
/// println!("Port: {}", port);
/// println!("Cluster: {}", cluster);
/// ```
///
use network_interface::{NetworkInterface, NetworkInterfaceConfig};
use std::cell::RefCell;
use std::env;
use std::net::IpAddr;

use crate::com::config::ClusterConfig;

thread_local!(static TLS_META: RefCell<Option<Meta>> = RefCell::new(None));

#[derive(Debug, Clone)]
pub struct Meta {
    cluster_name: String,
    port: String,
    ip: String,
}

pub fn get_if_addr() -> String {
    if let Ok(if_adrs) = NetworkInterface::show() {
        for iface in if_adrs.iter() {
            for iface_addr in iface.addr.iter() {
                match iface_addr {
                    network_interface::Addr::V4(v4) => {
                        if !v4.ip.is_unspecified() && !v4.ip.is_loopback() {
                            return v4.ip.to_string();
                        }
                    }
                    _default => {
                        continue;
                    }
                }
            }
        }
    }
    // get from env
    if let Ok(host_ip) = env::var("HOST") {
        if let Ok(addr) = host_ip.parse::<IpAddr>() {
            if !addr.is_unspecified() && addr.is_ipv4() && !addr.is_loopback() {
                return addr.to_string();
            }
        }
    }
    "127.0.0.1".to_string()
}

pub fn load_meta(cc: ClusterConfig, ip: Option<String>) -> Meta {
    let port = cc
        .listen_addr
        .split(':')
        .nth(1)
        .expect("listen_addr must contains port")
        .to_string();

    let ip = ip.unwrap_or_else(get_if_addr);

    Meta {
        cluster_name: cc.name,
        port,
        ip,
    }
}

pub fn meta_init(meta: Meta) {
    TLS_META.with(|gkd| {
        let mut handler = gkd.borrow_mut();
        handler.replace(meta);
    });
}

pub fn get_ip() -> String {
    TLS_META.with(|gkd| {
        gkd.borrow()
            .as_ref()
            .map(|x| x.ip.clone())
            .expect("get_ip must be called after init")
    })
}

pub fn get_port() -> String {
    TLS_META.with(|gkd| {
        gkd.borrow()
            .as_ref()
            .map(|x| x.port.clone())
            .expect("get_ip must be called after init")
    })
}

pub fn get_cluster() -> String {
    TLS_META.with(|gkd| {
        gkd.borrow()
            .as_ref()
            .map(|x| x.cluster_name.clone())
            .expect("get_ip must be called after init")
    })
}
