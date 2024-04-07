mod back;
// Path: src/proxy/standalone/back.rs

mod fnv;
// Path: src/proxy/standalone/fnv.rs

mod front;
// Path: src/proxy/standalone/front.rs

mod ketama;
// Path: src/proxy/standalone/ketama.rs

mod parser;
// Path: src/proxy/standalone/parser.rs

use crossbeam_channel::{bounded, Sender};
use crossbeam_utils::sync::{ShardedLock, ShardedLockReadGuard, ShardedLockWriteGuard};
use futures::StreamExt;
use log::{debug, error, info, warn};
use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    process,
    sync::Arc,
    time::Duration,
};
use tokio::{net::TcpStream, task::JoinHandle};
use tokio_util::codec::Decoder;

use crate::{
    com::{
        config::{
            create_reuse_port_listener, get_host_by_name, CacheType, ClusterConfig,
            CODE_PORT_IN_USE,
        },
        AsError,
    },
    metrics::front_conn_incr,
    protocol::{mc, redis},
    proxy::{
        standalone::{
            back::{Back, BlackHole},
            front::Front,
            ketama::HashRing,
            parser::ServerLine,
        },
        Request,
    },
    utils::helper::get_runtime_handle,
};

pub struct StandaloneCluster<T> {
    pub cc: ClusterConfig,

    hash_tag: Vec<u8>,
    auth: String,

    ring: RingKeeper<T>,
}

impl<T> StandaloneCluster<T>
where
    T: Request + Send + Sync + 'static,
{
    pub(crate) fn new(cc: ClusterConfig) -> Result<StandaloneCluster<T>, AsError> {
        let cluster = StandaloneCluster {
            cc: cc.clone(),
            hash_tag: cc
                .hash_tag
                .clone()
                .map(|x| x.into_bytes())
                .unwrap_or_default(),
            auth: cc.auth.clone(),
            ring: RingKeeper::new(),
        };

        cluster.init(cc)
    }

    fn init(mut self, cc: ClusterConfig) -> Result<StandaloneCluster<T>, AsError> {
        let parsed_servers = ServerLine::parse_servers(&cc.servers)?;
        let (nodes, alias, weights) = ServerLine::split_spots(&parsed_servers);

        let alias_map: HashMap<String, String> =
            alias.clone().into_iter().zip(nodes.clone()).collect();

        // let alias_rev: HashMap<String, String> = alias_map
        //     .iter()
        //     .map(|(x, y)| (y.clone(), x.clone()))
        //     .collect();

        let spots_map: HashMap<String, usize> = if alias.is_empty() {
            nodes.clone().into_iter().zip(weights.clone()).collect()
        } else {
            alias.clone().into_iter().zip(weights.clone()).collect()
        };

        let hash_ring = if alias.is_empty() {
            HashRing::new(nodes, weights)?
        } else {
            HashRing::new(alias, weights)?
        };

        let addrs: HashSet<_> = if !alias_map.is_empty() {
            alias_map.values().map(|x| x.to_string()).collect()
        } else {
            spots_map.keys().map(|x| x.to_string()).collect()
        };

        let old_addrs = self.ring.get().addrs();
        let new_addrs = addrs.difference(&old_addrs);
        let unused_addrs = old_addrs.difference(&addrs);

        for addr in new_addrs {
            self.connect(addr);
        }

        for addr in unused_addrs {
            self.ring.get_mut().remove_conn(addr);
        }

        self.cc = cc;
        self.ring.get_mut().coordinates = hash_ring;
        self.ring.alias = alias_map;
        self.ring.spots = spots_map;

        Ok(self)
    }

    pub(crate) fn run(self) -> JoinHandle<()> {
        let addr = self
            .cc
            .listen_addr
            .parse::<SocketAddr>()
            .expect("Listening address must be OK here");

        get_runtime_handle().spawn(async move {
            let listener = match create_reuse_port_listener(addr) {
                Ok(listener) => listener,
                Err(err) => {
                    error!("fail to create listener due to {}", err);
                    process::exit(CODE_PORT_IN_USE);
                }
            };

            info!("proxy is listening on {}", addr);

            let timeout = self.cc.timeout;
            let name = self.cc.name;

            loop {
                match listener.accept().await {
                    Ok((socket, addr)) => {
                        debug!("accepting connection from client at {}", addr);
                        if socket.set_nodelay(true).is_err() {
                            warn!(" cluster {} failed to set nodelay for {}", name, addr);
                        }

                        let codec = T::FrontCodec::default();
                        let (sink, stream) = codec.framed(socket).split();

                        let front = Front::new(
                            addr.to_string(),
                            self.hash_tag.clone(),
                            self.ring.clone(),
                            stream,
                            sink,
                            Duration::from_millis(timeout.unwrap_or(1000)),
                        );
                        get_runtime_handle().spawn(front);
                        front_conn_incr();
                    }
                    Err(err) => {
                        error!("fail to accept connection due to {}", err);
                        break;
                    }
                }
            }
        })
    }

    pub(crate) fn connect(&mut self, addr: &str) {
        debug!("trying to connect to {}", addr);

        self.ring.get_mut().remove_conn(addr);
        match connect(addr, Duration::from_millis(self.cc.timeout.unwrap_or(1000))) {
            Ok(sender) => {
                if !self.auth.is_empty() {
                    let auth_cmd = T::auth_request(&self.auth);
                    let _ = sender.send(auth_cmd);
                }

                self.ring.get_mut().insert_conn(addr, sender);
            }
            Err(err) => {
                error!("fail to connect to {} due {:?}", addr, err);
            }
        }
    }

    //     fn has_alias(&self) -> bool {
    //         !self.alias.borrow().is_empty()
    //     }

    //     fn get_node(&self, name: String) -> String {
    //         if !self.has_alias() {
    //             return name;
    //         }

    //         self.alias
    //             .borrow()
    //             .get(&name)
    //             .expect("alias name must exists")
    //             .to_string()
    //     }

    //     pub(crate) fn add_node(&self, name: String) -> Result<(), AsError> {
    //         if let Some(weight) = self.spots.borrow().get(&name).cloned() {
    //             let addr = self.get_node(name.clone());
    //             let mut conn = connect(&addr, self.cc.read_timeout, self.cc.write_timeout)?;

    //             if self.auth != "" {
    //                 let mut auth_cmd = T::auth_request(&self.auth);
    //                 // auth_cmd.reregister(task::current());
    //                 let _ = conn.start_send(auth_cmd);
    //             }

    //             self.conns.borrow_mut().insert(&addr, conn);
    //             self.ring.borrow_mut().add_node(name, weight);
    //         }
    //         Ok(())
    //     }

    //     pub(crate) fn remove_node(&self, name: String) {
    //         self.ring.borrow_mut().del_node(&name);
    //         let node = self.get_node(name);
    //         if self.conns.borrow_mut().remove(&node).is_some() {
    //             info!("dropping backend connection of {} due active delete", node);
    //         }
    //     }
}

// RingKeeper is a convenient wrapper around the ring to make it easier to access the ring
#[derive(Clone)]
struct RingKeeper<T> {
    ring: Arc<ShardedLock<Ring<T>>>,

    spots: HashMap<String, usize>,
    alias: HashMap<String, String>,
}

impl<T> RingKeeper<T> {
    fn new() -> Self {
        RingKeeper {
            ring: Arc::new(ShardedLock::new(Ring::<T>::new())),
            spots: HashMap::new(),
            alias: HashMap::new(),
        }
    }

    fn get(&self) -> ShardedLockReadGuard<Ring<T>> {
        self.ring.read().unwrap()
    }

    fn get_mut(&self) -> ShardedLockWriteGuard<Ring<T>> {
        self.ring.write().unwrap()
    }

    fn get_sender(&self, hash: u64) -> Option<Sender<T>> {
        debug!(
            "trying to find a backend node connection with hash {}",
            hash.to_string()
        );
        match self.get().coordinates.get_node(hash) {
            Some(node_name) => match self.get().get_inner(self.alias_or_default(node_name)) {
                Some(conn) => {
                    debug!(
                        "found node {} with addr {} for hash {}",
                        node_name,
                        conn.addr,
                        hash.to_string()
                    );
                    Some(conn.sender.clone())
                }
                None => {
                    error!(
                        "node {} does not have any connection on the ring",
                        node_name
                    );
                    None
                }
            },
            None => {
                error!("no node found on the ring for hash {}", hash.to_string());
                None
            }
        }
    }

    fn alias_or_default<'a>(&'a self, node_name: &'a str) -> &str {
        match self.alias.is_empty() {
            true => node_name,
            false => self
                .alias
                .get(node_name)
                .expect("alias must exists")
                .as_str(),
        }
    }
}

struct Ring<T> {
    coordinates: HashRing,
    inner: HashMap<String, Conn<T>>,
}

impl<T> Ring<T> {
    fn new() -> Self {
        Ring {
            coordinates: HashRing::empty(),
            inner: HashMap::new(),
        }
    }

    fn addrs(&self) -> HashSet<String> {
        self.inner.keys().cloned().collect()
    }

    fn get_inner(&self, s: &str) -> Option<&Conn<T>> {
        self.inner.get(s)
    }

    // fn get_inner_mut(&mut self, s: &str) -> Option<&mut Conn<T>> {
    //     self.inner.get_mut(s)
    // }

    fn remove_conn(&mut self, addr: &str) -> Option<Conn<T>> {
        self.inner.remove(addr)
    }

    fn insert_conn(&mut self, s: &str, sender: Sender<T>) {
        let conn = Conn {
            addr: s.to_string(),
            sender,
        };
        self.inner.insert(s.to_string(), conn);
    }
}

struct Conn<T> {
    addr: String,
    sender: Sender<T>,
}

fn connect<T>(node: &str, resp_timeout: Duration) -> Result<Sender<T>, AsError>
where
    T: Request + Send + 'static,
{
    let node_addr = node.to_string();
    let node_new = node_addr.clone();

    // TODO: the buffer size should be configurable
    let (tx, rx) = bounded(1024 * 8);

    let addr = get_host_by_name(node_addr.as_str()).expect("Socket address must be OK here");
    let report_addr = format!("{:?}", &addr);

    get_runtime_handle().spawn(async move {
        let connection = TcpStream::connect(addr).await.map_err(|err| {
            error!("fail to connect ot backend {} due to {}", report_addr, err);
            AsError::SystemError
        });
        match connection {
            Ok(socket) => {
                info!("connected to backend {}", report_addr);

                let codec = T::BackCodec::default();
                let (sink, stream) = codec.framed(socket).split();
                let backend = Back::new(node_new, rx, sink, stream, resp_timeout);
                get_runtime_handle().spawn(backend);
            }
            Err(_) => {
                let black_hole = BlackHole::new(node_new, rx);
                get_runtime_handle().spawn(black_hole);
            }
        }
    });

    Ok(tx)
}

pub fn spawn(cc: ClusterConfig) -> JoinHandle<()> {
    match cc.cache_type {
        CacheType::Redis => StandaloneCluster::<redis::Cmd>::new(cc)
            .expect("cluster encountered an error")
            .run(),
        CacheType::Memcache | CacheType::MemcacheBinary => StandaloneCluster::<mc::Cmd>::new(cc)
            .expect("cluster encountered an error")
            .run(),
        _ => {
            unreachable!("other cache types has to be check before calling spawn")
        }
    }
}
