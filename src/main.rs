use clap::{command, Parser};
use crossbeam_utils::sync::WaitGroup;
use librepust::{
    init_metrics_instruments, metrics_thread_incr, spawn, spawn_metrics, spawn_worker, CacheType,
    Config,
};
use log::{info, warn};
use std::thread;

/// Simple program to greet a person
#[derive(Parser, Debug, Clone)]
#[command(
    version,
    about = "Repust Redis/Memcached proxy server",
    long_about = "Repust is a Redis/Memcached proxy server focusing on high performance and availability."
)]
struct Args {
    /// App name, used for overriding the default app name in telemetry
    #[clap(short, long, default_value = "repust")]
    app_name: String,

    /// Config file path
    #[clap(short, long, default_value = "config.toml")]
    config_file_addr: String,

    /// Port for exposing metrics
    #[clap(short, long, default_value = "9001")]
    metrics_port: usize,
}

fn main() {
    let args: Args = Args::parse();

    env_logger::init();

    // reading config from file
    let cfg = Config::load(args.config_file_addr.clone())
        .expect("fail to load config file. make sure the file is exists and formatted correctly");

    // println!("use config : {:?}", cfg);
    assert!(
        !cfg.clusters.is_empty(),
        "clusters is absent of config file"
    );
    assert!(
        !cfg.log.level.is_empty(),
        "log level is absent of config file"
    );
    assert!(
        !cfg.log.directory.is_empty(),
        "log directory is absent of config file"
    );
    assert!(
        !cfg.log.file_name.is_empty(),
        "log file_name is absent of config file"
    );
    assert!(
        cfg.metrics.port != 0,
        "metrics port is absent of config file"
    );

    // blocking initiation of metrics instruments as they are needed asynchronously through out the program
    let registry = init_metrics_instruments(args.app_name);

    thread::spawn(move || {
        spawn_metrics(registry, args.metrics_port);
        metrics_thread_incr();
    });

    let wg = WaitGroup::new();
    for cluster in cfg.clusters.into_iter() {
        if cluster.servers.is_empty() {
            warn!(
                "fail to running cluster {} in addr {} due filed `servers` is empty",
                cluster.name, cluster.listen_addr
            );
            continue;
        }

        if cluster.name.is_empty() {
            warn!(
                "fail to running cluster {} in addr {} due filed `name` is empty",
                cluster.name, cluster.listen_addr
            );
            continue;
        }

        info!(
            "starting repust powered cluster {} in addr {}",
            cluster.name, cluster.listen_addr
        );

        let wg = wg.clone();
        thread::spawn(move || {
            match cluster.cache_type {
                CacheType::Redis | CacheType::Memcache | CacheType::MemcacheBinary => {
                    spawn_worker(&cluster, spawn);
                }
                _ => {
                    todo!("not support yet");
                }
            }
            // one parent thread for each cluster
            metrics_thread_incr();
            drop(wg);
        });
    }

    wg.wait();
}
