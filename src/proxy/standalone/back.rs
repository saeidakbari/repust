use crossbeam_channel::{Receiver, RecvTimeoutError};
use futures::{Future, Sink, Stream};
use log::{debug, error, info, warn};
use pin_project::pin_project;
use std::time::Duration;
use std::{
    pin::Pin,
    task::{Context, Poll},
};

use crate::{com::AsError, proxy::Request};

const DOWNSTREAM_MAX_POLL_ERROR: u8 = 10;

// CHANNEL_FETCH_TIMEOUT is the timeout to fetch the command from the channel.
// using the timeout, we can avoid the blocking of the task and it can be yielded back to the runtime.
// meanwhile preventing the task from instant wakeup and bruting the CPU usage.
const CHANNEL_FETCH_TIMEOUT: Duration = Duration::from_secs(1);

#[pin_project]
pub struct Back<T, S, R>
where
    T: Request,
    S: Sink<T, Error = AsError>,
    R: Stream<Item = Result<T::Reply, AsError>>,
{
    // conn_addr is the address of the backend server
    conn_addr: String,

    // store is the request which is waiting for the response
    // store is None if there is no request available from the front
    store: Option<T>,

    // input is the channel which receives the request from the front
    input: Receiver<T>,

    // downstream is the sink which sends the request to the back
    #[pin]
    downstream: S,

    // upstream is the stream which receives the response from the back
    #[pin]
    upstream: R,

    // resp_timeout is the maximum time to wait for the response
    resp_timeout: Duration,

    // downstream_poll_error is the counter to record the poll error of the downstream
    // if the counter is greater than DOWNSTREAM_MAX_POLL_ERROR, the backend is considered as unstable
    // and the backend will be closed
    downstream_poll_error: u8,

    // sub_cmds is the stack to store the sub commands
    sub_cmds: Vec<T>,

    // delayed is the number of delayed commands which should be skipped in the case of
    // any late reply received from the backend
    delayed: u32,
}

impl<T, S, R> Back<T, S, R>
where
    T: Request,
    S: Sink<T, Error = AsError>,
    R: Stream<Item = Result<T::Reply, AsError>>,
{
    pub fn new(
        conn_addr: String,
        input: Receiver<T>,
        downstream: S,
        upstream: R,
        read_timeout: Duration,
    ) -> Self {
        Back {
            conn_addr,
            store: None,
            input,
            downstream,
            upstream,
            resp_timeout: read_timeout,
            downstream_poll_error: 0,
            sub_cmds: Vec::new(),
            delayed: 0,
        }
    }
}

impl<T, S, R> Future for Back<T, S, R>
where
    T: Request,
    S: Sink<T, Error = AsError>,
    R: Stream<Item = Result<T::Reply, AsError>>,
{
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.as_mut().project();
        let store = this.store;

        let mut downstream = this.downstream;
        let upstream = this.upstream;
        let delayed = this.delayed;

        if store.is_none() {
            match this.sub_cmds.is_empty() {
                true => match this.input.recv_timeout(CHANNEL_FETCH_TIMEOUT) {
                    Ok(cmd) => {
                        match cmd.waker().is_some() {
                            true => {
                                debug!("backend {} received a command", this.conn_addr);

                                // if there are sub commands, push them into a stack and process them first at order.
                                // because the sub_cmds is a Vec, we need to reverse it to keep the incoming order.
                                if let Some(mut subs) = cmd.subs() {
                                    subs.iter_mut().for_each(|sub| {
                                        sub.register_waker(
                                            cmd.waker().expect("waker should not be empty here"),
                                        )
                                    });
                                    this.sub_cmds.extend(subs.into_iter().rev());
                                    *store = Some(
                                        this.sub_cmds.pop().expect("sub_cmds should not be empty"),
                                    );
                                } else {
                                    *store = Some(cmd);
                                }
                            }
                            false => debug!("dropping the command due to incorrect arrival path. waker was empty"),
                        }
                    }
                    Err(err) => match err {
                        RecvTimeoutError::Timeout => {
                            // wait for another wakeup
                        }
                        RecvTimeoutError::Disconnected => {
                            info!(
                                "channel from front is disconnected for backend {}",
                                this.conn_addr
                            );
                            return Poll::Ready(());
                        }
                    },
                },
                false => {
                    debug!(
                        "backend {} process the already available sub command",
                        this.conn_addr
                    );

                    // TODO: sub command error chain check
                    // sub commands should be handled in a more efficient way.
                    // if any sub command is failed, the whole command should be failed.
                    *store = Some(this.sub_cmds.pop().expect("sub_cmds should not be empty"));
                }
            }
        }

        if let Some(cmd) = store.take() {
            match cmd.get_sent_time() {
                Some(sent_time) => {
                    if sent_time.elapsed() > *this.resp_timeout {
                        error!("backend {} read timeout", this.conn_addr);
                        cmd.set_error(&AsError::CmdTimeout);
                        *delayed += 1;
                        *store = None;
                    } else {
                        *store = Some(cmd);
                    }
                }
                None => match downstream.as_mut().poll_ready(cx) {
                    Poll::Ready(Ok(())) => {
                        debug!("backend {} sent a command", this.conn_addr);
                        cmd.mark_sent();
                        let waited_cmd = cmd.clone();
                        if let Err(err) = downstream.as_mut().start_send(cmd) {
                            error!(
                                "backend {} failed to send a command due to {}",
                                this.conn_addr, err
                            );
                            waited_cmd.set_error(&AsError::ProxyFail);
                            *store = None;
                        } else {
                            let _ = downstream.poll_flush(cx);
                            *store = Some(waited_cmd);
                        }
                    }
                    Poll::Ready(Err(err)) => {
                        warn!(
                            "backend {} failed to send a command due to {}",
                            this.conn_addr, err
                        );
                        if cmd.can_cycle() {
                            cmd.add_cycle();
                        } else {
                            cmd.set_error(&AsError::ProxyFail);
                            *store = None;
                        }

                        *this.downstream_poll_error += 1;
                        if *this.downstream_poll_error > DOWNSTREAM_MAX_POLL_ERROR {
                            error!("backend {} is not stable to send commands", this.conn_addr);
                            return Poll::Ready(());
                        }
                    }
                    Poll::Pending => {
                        debug!("backend {} is not ready yet", this.conn_addr);
                        *store = Some(cmd);
                    }
                },
            }
        }

        if let Some(cmd) = &store {
            match upstream.poll_next(cx) {
                Poll::Ready(Some(may_reply)) => match may_reply {
                    Ok(reply) => {
                        debug!("backend {} received a reply", this.conn_addr);

                        if *delayed > 0 {
                            warn!(
                                "backend {} received a late reply, skipping {} delayed commands",
                                this.conn_addr, delayed
                            );
                            *delayed -= 1;
                        } else {
                            cmd.set_reply(reply);
                            *store = None;
                        }
                    }
                    Err(err) => {
                        debug!("backend {} received an error", this.conn_addr);
                        cmd.set_error(&err);
                        *store = None;
                    }
                },
                Poll::Ready(None) => {
                    debug!("backend {} is disconnected", this.conn_addr);
                    return Poll::Ready(());
                }
                Poll::Pending => {}
            }
        }

        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

pub struct BlackHole<T>
where
    T: Request,
{
    addr: String,

    // input is the channel which receives the request from the front
    input: Receiver<T>,
}

impl<T> BlackHole<T>
where
    T: Request,
{
    pub fn new(addr: String, input: Receiver<T>) -> BlackHole<T> {
        BlackHole { addr, input }
    }
}

impl<T> Future for BlackHole<T>
where
    T: Request,
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.input.recv_timeout(CHANNEL_FETCH_TIMEOUT) {
            Ok(cmd) => {
                info!("backend BlackHole clear the connection for {}", self.addr);
                cmd.set_error(&AsError::BackendClosedError(self.addr.clone()));
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Err(err) => match err {
                RecvTimeoutError::Timeout => {
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
                RecvTimeoutError::Disconnected => {
                    error!(
                        "backend BlackHole channel is disconnected for {} due to {}",
                        self.addr, err
                    );
                    Poll::Ready(())
                }
            },
        }
    }
}
