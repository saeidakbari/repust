mod utils;
// Path: src/utils.rs

mod protocol;
// Path: src/protocol.rs

mod proxy;
// Path: src/proxy.rs

mod com;
// Path: src/com.rs

mod metrics;
// Path: src/metrics.rs

use log::{error, info};
use prometheus::Registry;
use tokio::{runtime::Builder, task::JoinHandle};

use crate::{
    com::{config::ClusterConfig, meta::load_meta},
    metrics::init as metrics_init,
    protocol::mc::init_memcached_text_finder,
};

pub use crate::com::config::{CacheType, Config};
pub use crate::metrics::{
    init_instruments as init_metrics_instruments, thread_incr as metrics_thread_incr,
    thread_incr_by as metrics_thread_incr_by,
};
use crate::protocol::redis::init_redis_supported_cmds;
pub use crate::proxy::standalone::spawn;

const DEFAULT_THREAD_COUNT: usize = 4;

pub fn spawn_worker<T>(cc: &ClusterConfig, spawn_fn: T)
where
    T: Fn(ClusterConfig) -> JoinHandle<()> + Copy + Send + 'static,
{
    match cc.cache_type {
        CacheType::Redis | CacheType::RedisCluster => {
            init_redis_supported_cmds();
        }
        CacheType::Memcache | CacheType::MemcacheBinary => {
            init_memcached_text_finder();
        }
    }

    init_redis_supported_cmds();

    let addr = match !cc.listen_addr.is_empty() {
        true => Some(cc.listen_addr.clone()),
        false => None,
    };

    let meta = load_meta(cc.clone(), addr);

    info!("setup meta info with {:?}", meta);

    let cc = cc.clone();
    let runtime = Builder::new_multi_thread()
        .thread_name(cc.name.clone())
        .worker_threads(cc.thread.unwrap_or(DEFAULT_THREAD_COUNT))
        .enable_all()
        .build()
        .unwrap();

    metrics_thread_incr_by(cc.thread.unwrap() as u64);

    runtime.block_on(async move {
        let _ = spawn_fn(cc).await;
    });
}

pub fn spawn_metrics(registry: Registry, port: usize) {
    let runtime = Builder::new_current_thread()
        .thread_name("metrics")
        .enable_all()
        .build()
        .unwrap();

    metrics_thread_incr();

    runtime.block_on(async move {
        match metrics_init(registry, port) {
            Ok(jh) => {
                info!("metrics server started at port {}", port);
                jh.await.unwrap();
            }
            Err(err) => {
                error!(
                    "fail to start metrics server at port {} due to {}",
                    port, err
                );
            }
        }
    });
}
