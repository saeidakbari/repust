pub mod tracker;
// Path: src/metrics/tracker.rs

mod measurer;
// Path: src/metrics/measurer.rs

use axum::extract::State;
use axum::{routing::get, Router};
use log::{error, info};
use opentelemetry::metrics::{
    Counter, Histogram, MeterProvider as _, ObservableGauge, UpDownCounter,
};
use opentelemetry::KeyValue;
use opentelemetry_sdk::metrics::MeterProvider;
use opentelemetry_sdk::Resource;
use prometheus::{Registry, TextEncoder};
use std::net::SocketAddr;
use std::sync::OnceLock;
use tokio::task::JoinHandle;

use crate::com::{config::create_reuse_port_listener, AsError};
use crate::metrics::measurer::Measurer;

// REPUST_METER_NAME is the name of the meter used to create the global metrics.
const REPUST_METER_NAME: &str = "global";

// METER_PROVIDER is the global meter provider, it is used to create the global metrics.
static METER_PROVIDER: OnceLock<MeterProvider> = OnceLock::new();

// REPUST_CONNECTIONS is a global connection counter, it is used to count the global connections.
static REPUST_CONNECTIONS: OnceLock<UpDownCounter<i64>> = OnceLock::new();

// REPUST_MEMORY is a global memory usage gauge, it is used to count the global memory usage.
static REPUST_MEMORY: OnceLock<ObservableGauge<f64>> = OnceLock::new();

// REPUST_CPU is a global cpu usage gauge, it is used to count the global cpu usage.
static REPUST_CPU: OnceLock<ObservableGauge<f64>> = OnceLock::new();

// REPUST_THREADS is a global thread counter, it is used to count the global threads.
static REPUST_THREADS: OnceLock<Counter<u64>> = OnceLock::new();

// REPUST_GLOBAL_ERROR is a global error counter, it is used to count the global errors.
static REPUST_GLOBAL_ERROR: OnceLock<Counter<u64>> = OnceLock::new();

// REPUST_TOTAL_TIMER is a global total timer histogram, it is used to count the global total timer.
static REPUST_TOTAL_TIMER: OnceLock<Histogram<f64>> = OnceLock::new();

// REPUST_REMOTE_TIMER is a global remote timer histogram, it is used to count the global remote timer.
static REPUST_REMOTE_TIMER: OnceLock<Histogram<f64>> = OnceLock::new();

// front_conn_incr increments the global connection counter.
pub fn front_conn_incr() {
    REPUST_CONNECTIONS
        .get()
        .unwrap()
        .add(1, &[KeyValue::new("connection_type", "inbound")])
}

// front_conn_decr decrements the global connection counter.
pub fn front_conn_decr() {
    REPUST_CONNECTIONS
        .get()
        .unwrap()
        .add(-1, &[KeyValue::new("connection_type", "inbound")])
}

// global_error_incr increments the global error counter.
pub fn global_error_incr() {
    REPUST_GLOBAL_ERROR.get().unwrap().add(1, &[]);
}

// thread_incr increments the global thread counter.
pub fn thread_incr() {
    REPUST_THREADS.get().unwrap().add(1, &[]);
}

// thread_incr_by increments the global thread counter by the given count.
pub fn thread_incr_by(count: u64) {
    REPUST_THREADS.get().unwrap().add(count, &[]);
}

fn init_meter_provider(app_name: String, registry: Registry) {
    let exporter = opentelemetry_prometheus::exporter()
        .with_registry(registry)
        .build()
        .expect("creating exporter should not fail");

    METER_PROVIDER
        .set(
            MeterProvider::builder()
                .with_reader(exporter)
                .with_resource(Resource::new([KeyValue::new("service.name", app_name)]))
                .build(),
        )
        .expect("creating meter provider should not fail");
}

async fn exporter_handler(state: State<Registry>) -> String {
    let encoder = TextEncoder::new();
    encoder.encode_to_string(&state.gather()).unwrap()
}

pub fn init_instruments(app_name: String) -> Registry {
    let registry = prometheus::Registry::new();

    init_meter_provider(app_name, registry.clone());
    let meter = METER_PROVIDER.get().unwrap().meter(REPUST_METER_NAME);

    REPUST_CONNECTIONS
        .set(
            meter
                .i64_up_down_counter("repust.connection")
                .with_description("total connection counter")
                .init(),
        )
        .expect("initializing metric should not fail");

    REPUST_MEMORY
        .set(
            meter
                .f64_observable_gauge("repust.memory")
                .with_description("total memory usage")
                .init(),
        )
        .expect("initializing metric should not fail");

    REPUST_CPU
        .set(
            meter
                .f64_observable_gauge("repust.cpu")
                .with_description("total cpu usage")
                .init(),
        )
        .expect("initializing metric should not fail");

    REPUST_THREADS
        .set(
            meter
                .u64_counter("repust.threads")
                .with_description("total thread counter")
                .init(),
        )
        .expect("initializing metric should not fail");

    REPUST_GLOBAL_ERROR
        .set(
            meter
                .u64_counter("repust.error")
                .with_description("total error counter")
                .init(),
        )
        .expect("initializing metric should not fail");

    REPUST_TOTAL_TIMER
        .set(
            meter
                .f64_histogram("repust.total_timer")
                .with_description("set up each cluster command proxy total timer")
                .init(),
        )
        .expect("initializing metric should not fail");

    REPUST_REMOTE_TIMER
        .set(
            meter
                .f64_histogram("repust.remote_timer")
                .with_description("set up each cluster command proxy remote timer")
                .init(),
        )
        .expect("initializing metric should not fail");

    registry
}

// TODO: use each cluster name for in-depth better observability
pub fn init(registry: Registry, port: usize) -> Result<JoinHandle<()>, AsError> {
    let measurer = Measurer::new(std::time::Duration::from_secs(10))
        .expect("initializing measurer should not fail");

    tokio::spawn(measurer);

    // TODO: add healthz route in the future
    let app = Router::new().route("/metrics", get(exporter_handler).with_state(registry));

    let addr = format!("0.0.0.0:{}", port);
    let socket = addr
        .parse::<SocketAddr>()
        .expect("parse socket address should not fail");

    match create_reuse_port_listener(socket) {
        Ok(listener) => {
            info!("listen http metrics port in addr {}", port);

            Ok(tokio::spawn(async move {
                axum::serve(listener, app)
                    .await
                    .expect("failed to serve metric on HTTP"); // Await the serve function call
            }))
        }
        Err(err) => {
            error!("fail to create reuse port listener due {}", err);
            Err(AsError::SystemError)
        }
    }
}
