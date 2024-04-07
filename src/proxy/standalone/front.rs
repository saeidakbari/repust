use crossbeam_channel::SendTimeoutError;
use futures::{Future, Sink, Stream};
use log::{debug, error};
use pin_project::{pin_project, pinned_drop};
use std::{
    collections::VecDeque,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use crate::{
    com::AsError,
    metrics::front_conn_decr,
    proxy::{
        standalone::{fnv::fnv1a64, RingKeeper},
        Request,
    },
};

const FRONTEND_MAX_POLL_ERROR: u8 = 10;

#[pin_project(PinnedDrop)]
pub struct Front<T, I, O>
where
    T: Request,
    O: Sink<T, Error = AsError>,
    I: Stream<Item = Result<T, AsError>>,
{
    // client is the name of the client, usually the address of the client
    client: String,

    // hash_tag ensures that multiple keys are allocated in the same hash slot.
    // This is useful for situations when multiple keys are stored in the same hash slot.
    hash_tag: Vec<u8>,

    // ring is the entire cluster information including addresses, connections and their associated sender channels.
    ring: RingKeeper<T>,

    // downstream here represent the stream which takes commands from the client.
    // Since the proxy is sat between clients and the backends is is act as a downstream to the clients.
    #[pin]
    downstream: I,

    #[pin]
    // upstream here represent the sink which sends the response to the client.
    upstream: O,

    // timeout is the time after which the request will be considered as failed
    timeout: Duration,

    // sent_queue is the queue which holds the requests which are sent to the back but not yet received the response.
    // This queue is used to check the reply of the requests on the order they were sent.
    sent_queue: VecDeque<T>,

    // upstream_poll_error is the counter to record the send error of the upstream
    upstream_poll_error: u8,
}

impl<T, I, O> Front<T, I, O>
where
    T: Request,
    O: Sink<T, Error = AsError>,
    I: Stream<Item = Result<T, AsError>>,
{
    pub fn new(
        client: String,
        hash_tag: Vec<u8>,
        ring: RingKeeper<T>,
        downstream: I,
        upstream: O,
        timeout: Duration,
    ) -> Self {
        Front {
            client,
            hash_tag,
            ring,
            downstream,
            upstream,
            timeout,
            sent_queue: VecDeque::new(),
            upstream_poll_error: 0,
        }
    }
}

impl<T, I, O> Future for Front<T, I, O>
where
    T: Request,
    O: Sink<T, Error = AsError>,
    I: Stream<Item = Result<T, AsError>>,
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let this = self.project();

        let downstream = this.downstream;
        let mut upstream = this.upstream;

        if let Some(cmd) = this.sent_queue.pop_front() {
            if cmd.is_done() {
                debug!("command is done, sending the reply to the client");

                // send the reply to the client
                match upstream.as_mut().poll_ready(cx) {
                    Poll::Ready(Ok(())) => {
                        if let Err(err) = upstream.as_mut().start_send(cmd) {
                            error!(
                                "frontend {} failed to send reply to client: {}",
                                this.client, err
                            );
                        } else {
                            let _ = upstream.poll_flush(cx);
                        }
                    }
                    Poll::Ready(Err(err)) => {
                        error!(
                            "frontend {} failed to send reply to client: {}",
                            this.client, err
                        );

                        *this.upstream_poll_error += 1;
                        if *this.upstream_poll_error > FRONTEND_MAX_POLL_ERROR {
                            error!(
                                "frontend {} is not stable to send replies, closing the connection",
                                this.client
                            );
                            return Poll::Ready(());
                        }
                    }
                    Poll::Pending => {}
                }
            } else {
                // push the command back to the sent queue to check the response later in order
                this.sent_queue.push_front(cmd);
            }
        }

        match downstream.poll_next(cx) {
            Poll::Ready(Some(may_cmd)) => {
                match may_cmd {
                    Ok(mut cmd) => {
                        // if the command is invalid or done, send it to the client for immediate response.
                        if cmd.valid() && !cmd.is_done() {
                            debug!("frontend received a command from client {}", this.client);

                            // register the waker to the command to wake up the task when the response is ready
                            cmd.register_waker(cx.waker().clone());

                            // find the output connection for the command based on the hash of the cmd key
                            let key_hash = cmd.key_hash("".as_bytes(), fnv1a64);
                            match this.ring.get_sender(key_hash) {
                                Some(output) => {
                                    // send the command to the back for processing
                                    // Note: cloning the cmd produces a new pointer to the same underlying data because of
                                    // using Rc in the cmd interior. So, it is not an expensive operation.
                                    match output.send_timeout(cmd.clone(), *this.timeout) {
                                        Ok(_) => {
                                            debug!(
                                                "frontend {} forwarded command to back",
                                                this.client
                                            )
                                        }
                                        Err(err) => match err {
                                            SendTimeoutError::Timeout(cmd) => {
                                                error!(
                                                    "frontend {} faced timeout to forward command",
                                                    this.client
                                                );
                                                cmd.set_error(&AsError::CmdTimeout);
                                            }
                                            SendTimeoutError::Disconnected(cmd) => {
                                                error!(
                                                    "frontend {} has no backend consumer",
                                                    this.client
                                                );
                                                cmd.set_error(&AsError::ClusterFailDispatch);
                                            }
                                        },
                                    }
                                }
                                None => {
                                    error!(
                                        "frontend {} failed to find output channel for the command based on cmd hash",
                                        this.client
                                    );
                                    cmd.set_error(&AsError::ClusterFailDispatch);
                                }
                            };
                        }
                        // push the command to the sent queue to check the response later in order
                        this.sent_queue.push_back(cmd);

                        // Wake the task until there are no values to be received from stream.
                        // After stream returns Pending, waker is automatically registered to wake up the task in the
                        // case of new value is ready to be received.
                        cx.waker().wake_by_ref();
                    }
                    Err(err) => {
                        error!(
                            "frontend {} failed to receive command from client due to: {}",
                            this.client, err
                        );
                    }
                }
            }
            Poll::Ready(None) => {
                debug!("frontend terminated for client {}", this.client);
                return Poll::Ready(());
            }
            Poll::Pending => {}
        }

        Poll::Pending
    }
}

#[pinned_drop]
impl<T, I, O> PinnedDrop for Front<T, I, O>
where
    T: Request,
    O: Sink<T, Error = AsError>,
    I: Stream<Item = Result<T, AsError>>,
{
    fn drop(self: Pin<&mut Self>) {
        debug!("frontend dropped for client {}", self.client);
        front_conn_decr();
    }
}
