pub mod standalone;
// Path: src/proxy/standalone.rs

use std::task::Waker;
use std::time::Instant;
use tokio_util::codec::{Decoder, Encoder};

use crate::com::AsError;
use crate::protocol::IntoReply;

pub trait Request: Clone {
    type Reply: Clone + IntoReply<Self::Reply> + From<AsError>;

    type FrontCodec: Decoder<Item = Self, Error = AsError>
        + Encoder<Self, Error = AsError>
        + Default
        + Send;

    type BackCodec: Decoder<Item = Self::Reply, Error = AsError>
        + Encoder<Self, Error = AsError>
        + Default
        + Send;

    fn ping_request() -> Self;
    fn auth_request(auth: &str) -> Self;
    // fn reregister(&mut self, task: Task);

    fn key_hash(&self, hash_tag: &[u8], hasher: fn(&[u8]) -> u64) -> u64;

    fn subs(&self) -> Option<Vec<Self>>;

    fn mark_total(&self);
    fn mark_sent(&self);

    fn is_done(&self) -> bool;
    fn is_error(&self) -> bool;

    fn add_cycle(&self);
    fn can_cycle(&self) -> bool;

    fn valid(&self) -> bool;

    fn register_waker(&mut self, waker: Waker);
    fn waker(&self) -> Option<Waker>;

    fn set_reply<R: IntoReply<Self::Reply>>(&self, t: R);
    fn set_error(&self, t: &AsError);

    fn get_sent_time(&self) -> Option<Instant>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Redirect {
    Move { slot: usize, to: String },
    Ask { slot: usize, to: String },
}

impl Redirect {
    pub(crate) fn is_ask(&self) -> bool {
        matches!(self, Redirect::Ask { .. })
    }
}
