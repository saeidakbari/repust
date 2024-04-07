use futures::Future;
use log::{debug, warn};
use std::{
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, Pid, RefreshKind};
use tokio::time::{self, Interval};

use crate::com::AsError;
use crate::metrics::{REPUST_CPU, REPUST_MEMORY};

pub struct Measurer {
    // pid is the process id of the current repust proxy
    pid: Pid,

    // interval is the interval at which the metrics will be measured
    interval: Interval,
}

impl Measurer {
    // new creates a new Measurer with the given measure_interval and pid
    pub fn new(measure_interval: Duration) -> Result<Self, AsError> {
        let pid = match sysinfo::get_current_pid() {
            Ok(pid) => pid,
            Err(err) => {
                warn!("fail get pid of current repust due {}", err);
                return Err(AsError::SystemError);
            }
        };

        Ok(Measurer {
            pid,
            interval: time::interval(measure_interval),
        })
    }

    // measure_system measures the system metrics
    fn measure_system(&self) -> Result<(), AsError> {
        let mut system = sysinfo::System::new();
        system.refresh_specifics(
            RefreshKind::new()
                .with_cpu(CpuRefreshKind::new().with_cpu_usage())
                .with_memory(MemoryRefreshKind::new().with_ram()),
        );

        // First we update all information of our system struct.
        if !system.refresh_process(self.pid) {
            return Ok(());
        }

        match system.process(self.pid) {
            Some(proc) => {
                let cpu_usage = proc.cpu_usage() as f64;
                let memory_usage = proc.memory() as f64;
                REPUST_MEMORY.get().unwrap().observe(memory_usage, &[]);
                REPUST_CPU.get().unwrap().observe(cpu_usage, &[]);
                Ok(())
            }
            None => {
                warn!("fail to get process info of pid {}", self.pid);
                Err(AsError::SystemError)
            }
        }
    }
}

impl Future for Measurer {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        match self.interval.poll_tick(cx) {
            Poll::Ready(_) => {
                debug!("measuring system metrics");
                if let Err(err) = self.measure_system() {
                    warn!("fail to measure system metrics due {}", err);
                }
                cx.waker().wake_by_ref();
            }
            Poll::Pending => {} // do nothing
        }
        Poll::Pending
    }
}
