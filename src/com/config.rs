use log::{error, info};
use serde::Deserialize;
use socket2::{Domain, Socket, Type};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::net::SocketAddr;
use std::net::ToSocketAddrs;
use std::path::Path;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};

use crate::com::AsError;

const ENV_REPUST_DEFAULT_THREADS: &str = "REPUST_DEFAULT_THREAD";
const DEFAULT_FETCH_INTERVAL_MS: u64 = 30 * 60 * 1000;

pub const CODE_PORT_IN_USE: i32 = 1;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    #[serde(default)]
    pub log: LogConfig,

    #[serde(default)]
    pub metrics: MetricsConfig,

    #[serde(default)]
    pub clusters: Vec<ClusterConfig>,
}

impl Config {
    pub fn cluster(&self, name: &str) -> Option<ClusterConfig> {
        for cluster in &self.clusters {
            if cluster.name == name {
                return Some(cluster.clone());
            }
        }
        None
    }

    fn servers_map(&self) -> BTreeMap<String, BTreeSet<String>> {
        self.clusters
            .iter()
            .map(|x: &ClusterConfig| {
                (
                    x.name.clone(),
                    x.servers.iter().map(|s| s.to_string()).collect(),
                )
            })
            .collect()
    }

    pub fn reload_equals(&self, other: &Config) -> bool {
        self.servers_map() == other.servers_map()
    }

    pub fn valid(&self) -> Result<(), AsError> {
        Ok(())
    }

    pub fn load<P: AsRef<Path>>(p: P) -> Result<Config, AsError> {
        let data = fs::read_to_string(p.as_ref())?;
        info!("load config data {}", data);

        let mut cfg: Config = toml::from_str(&data)?;
        let thread = Config::load_thread_from_env();
        for cluster in &mut cfg.clusters[..] {
            if cluster.thread.is_none() {
                cluster.thread = Some(thread);
            }
        }
        Ok(cfg)
    }

    fn load_thread_from_env() -> usize {
        let thread_str = env::var(ENV_REPUST_DEFAULT_THREADS).unwrap_or_else(|_| "4".to_string());
        thread_str.parse::<usize>().unwrap_or(4)
    }
}

#[derive(Clone, Debug, Deserialize, Default)]
pub struct LogConfig {
    pub level: String,
    pub ansi: bool,
    pub stdout: bool,
    pub directory: String,
    pub file_name: String,
}

#[derive(Clone, Debug, Deserialize, Default)]
pub struct MetricsConfig {
    pub port: usize,
}

#[derive(Deserialize, Debug, Clone, Copy, Default)]
pub enum CacheType {
    #[serde(rename = "redis")]
    #[default]
    Redis,

    #[serde(rename = "memcache")]
    Memcache,

    #[serde(rename = "memcache_binary")]
    MemcacheBinary,

    #[serde(rename = "redis_cluster")]
    RedisCluster,
}

#[derive(Clone, Debug, Deserialize, Default)]
pub struct ClusterConfig {
    pub name: String,
    pub listen_addr: String,
    pub hash_tag: Option<String>,

    pub thread: Option<usize>,
    pub cache_type: CacheType,

    pub timeout: Option<u64>,

    #[serde(default)]
    pub servers: Vec<String>,

    // cluster special
    pub fetch_interval: Option<u64>,
    pub read_from_slave: Option<bool>,

    // proxy special
    pub ping_fail_limit: Option<u8>,
    pub ping_interval: Option<u64>,
    pub ping_success_interval: Option<u64>,

    // dead codes

    // command not support now
    pub dial_timeout: Option<u64>,
    // dead option: not support other proto
    pub listen_proto: Option<String>,

    // dead option: always 1
    pub node_connections: Option<usize>,

    // password to connect to node, and for auth for client
    pub auth: String,
}

impl ClusterConfig {
    pub(crate) fn hash_tag_bytes(&self) -> Vec<u8> {
        self.hash_tag
            .as_ref()
            .map(|x| x.as_bytes().to_vec())
            .unwrap_or_default()
    }

    pub(crate) fn fetch_interval_ms(&self) -> u64 {
        self.fetch_interval.unwrap_or(DEFAULT_FETCH_INTERVAL_MS)
    }
}

#[cfg(windows)]
pub(crate) fn create_reuse_port_listener(addr: SocketAddr) -> Result<TcpListener, std::io::Error> {
    let socket = Socket::new(Domain::IPV4, Type::STREAM, None)?;

    socket.set_only_v6(false);
    socket
        .set_reuse_address(true)
        .expect("os not support SO_REUSEADDR");
    socket.bind(&socket2::SockAddr::from(addr));
    socket.listen(std::i32::MAX);

    TcpListener::from_std(socket.into())
}

#[cfg(not(windows))]
pub(crate) fn create_reuse_port_listener(addr: SocketAddr) -> Result<TcpListener, std::io::Error> {
    let socket = Socket::new(Domain::IPV4, Type::STREAM, None)?;

    let _ = socket.set_only_v6(false);
    socket
        .set_nonblocking(true)
        .expect("socket must be nonblocking");
    socket
        .set_reuse_address(true)
        .expect("os not support SO_REUSEADDR");
    socket
        .set_reuse_port(true)
        .expect("os not support SO_REUSEADDR");
    socket
        .bind(&socket2::SockAddr::from(addr))
        .expect("socket binding should be ok");
    socket
        .listen(std::i32::MAX)
        .expect("listening to socket should not return error");

    TcpListener::from_std(socket.into())
}

#[cfg(not(unix))]
#[inline]
pub fn set_read_write_timeout(
    sock: TcpStream,
    _rt: Option<u64>,
    _wt: Option<u64>,
) -> Result<TcpStream, AsError> {
    Ok(sock)
}

#[cfg(unix)]
#[inline]
pub fn set_read_write_timeout(
    sock: TcpStream,
    rt: Option<u64>,
    wt: Option<u64>,
) -> Result<TcpStream, AsError> {
    use std::os::unix::io::AsRawFd;
    use std::os::unix::io::FromRawFd;

    let nrt = rt.map(Duration::from_millis);
    let nwt = wt.map(Duration::from_millis);
    let fd = sock.as_raw_fd();

    let new_socket = unsafe { std::net::TcpStream::from_raw_fd(fd) };
    std::mem::forget(sock);

    new_socket.set_read_timeout(nrt)?;
    new_socket.set_write_timeout(nwt)?;
    let stream = TcpStream::from_std(new_socket)?;

    Ok(stream)
}

pub(crate) fn get_host_by_name(name: &str) -> Result<SocketAddr, AsError> {
    let mut iter: std::vec::IntoIter<SocketAddr> = name.to_socket_addrs().map_err(|err| {
        error!("fail to resolve addr to {} by {}", name, err);
        AsError::BadConfig("servers".to_string())
    })?;

    let addr = iter
        .next()
        .ok_or_else(|| AsError::BadConfig(format!("servers:{}", name)))?;

    Ok(addr)
}
