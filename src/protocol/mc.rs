pub mod msg;

use bytes::BytesMut;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::task::Waker;
use std::time::Instant;
use tokio_util::codec::{Decoder, Encoder};

use crate::com::AsError;
use crate::metrics::tracker::{remote_tracker, total_tracker, Tracker};
use crate::protocol::mc::msg::Message;
use crate::protocol::{CmdFlags, CmdType, IntoReply};
use crate::proxy::Request;
use crate::utils::helper::trim_hash_tag;

pub use crate::protocol::mc::msg::init_text_finder as init_memcached_text_finder;

const MAX_CYCLE: u8 = 1;

#[derive(Clone)]
pub struct Cmd {
    cmd: Arc<RwLock<Command>>,
    waker: Option<Waker>,
}

impl Drop for Cmd {
    fn drop(&mut self) {
        if !self.is_done() {
            self.set_error(&AsError::ProxyFail);
        }
    }
}

impl Request for Cmd {
    type Reply = Message;
    type FrontCodec = FrontCodec;
    type BackCodec = BackCodec;

    fn ping_request() -> Self {
        let cmd = Command {
            ctype: CmdType::Read,
            flags: CmdFlags::empty(),
            cycle: 0,

            req: Message::version_request(),
            reply: None,
            subs: None,

            total_tracker: None,
            remote_tracker: None,
        };
        Cmd {
            cmd: Arc::new(RwLock::new(cmd)),
            waker: None,
        }
    }

    // FIXME
    fn auth_request(_auth: &str) -> Self {
        let cmd = Command {
            ctype: CmdType::Auth,
            flags: CmdFlags::empty(),
            cycle: 0,

            req: Message::version_request(),
            reply: None,
            subs: None,

            total_tracker: None,
            remote_tracker: None,
        };
        Cmd {
            cmd: Arc::new(RwLock::new(cmd)),
            waker: None,
        }
    }

    fn key_hash(&self, hash_tag: &[u8], hasher: fn(&[u8]) -> u64) -> u64 {
        let cmd = self.take_cmd();
        let key = cmd.req.get_key();
        hasher(trim_hash_tag(key, hash_tag))
    }

    fn subs(&self) -> Option<Vec<Self>> {
        self.take_cmd().subs.clone()
    }

    fn is_done(&self) -> bool {
        if let Some(subs) = self.subs() {
            subs.iter().all(|x| x.is_done())
        } else {
            self.take_cmd().is_done()
        }
    }

    fn add_cycle(&self) {
        self.take_cmd_mut().add_cycle()
    }
    fn can_cycle(&self) -> bool {
        self.take_cmd().can_cycle()
    }

    fn is_error(&self) -> bool {
        self.take_cmd().is_error()
    }

    fn valid(&self) -> bool {
        true
    }

    fn register_waker(&mut self, waker: Waker) {
        self.waker = Some(waker);
    }

    fn waker(&self) -> Option<Waker> {
        self.waker.clone()
    }

    fn set_reply<R: IntoReply<Message>>(&self, t: R) {
        let reply = t.into_reply();
        self.take_cmd_mut().set_reply(reply);
    }

    fn set_error(&self, t: &AsError) {
        let reply: Message = t.into_reply();
        self.take_cmd_mut().set_error(reply);
    }

    fn mark_total(&self) {
        let timer = total_tracker();
        self.take_cmd_mut().total_tracker.replace(timer);
    }

    fn mark_sent(&self) {
        let timer = remote_tracker();
        self.take_cmd_mut().remote_tracker.replace(timer);
    }

    fn get_sent_time(&self) -> Option<Instant> {
        let mut c = self.take_cmd_mut();
        match c.remote_tracker.take() {
            Some(t) => {
                let s = t.start;
                c.remote_tracker = Some(t);
                Some(s)
            }
            None => None,
        }
    }
}

impl Cmd {
    fn from_msg(msg: Message) -> Cmd {
        let flags = CmdFlags::empty();
        let ctype = CmdType::Read;
        let sub_msgs = msg.mk_subs();

        let subs: Vec<_> = sub_msgs
            .into_iter()
            .map(|sub_msg| {
                let command = Command {
                    ctype,
                    flags,
                    cycle: 0,
                    req: sub_msg,
                    reply: None,
                    subs: None,

                    total_tracker: None,

                    remote_tracker: None,
                };
                Cmd {
                    cmd: Arc::new(RwLock::new(command)),
                    waker: None,
                }
            })
            .collect();
        let subs = if subs.is_empty() { None } else { Some(subs) };
        let command = Command {
            ctype: CmdType::Read,
            flags: CmdFlags::empty(),
            cycle: 0,
            req: msg,
            reply: None,
            subs,

            total_tracker: None,

            remote_tracker: None,
        };
        Cmd {
            cmd: Arc::new(RwLock::new(command)),
            waker: None,
        }
    }

    pub fn take_cmd(&self) -> RwLockReadGuard<Command> {
        self.cmd.read().unwrap()
    }

    pub fn take_cmd_mut(&self) -> RwLockWriteGuard<Command> {
        self.cmd.write().unwrap()
    }
}

impl From<Message> for Cmd {
    fn from(msg: Message) -> Cmd {
        Cmd::from_msg(msg)
    }
}

#[allow(unused)]
pub struct Command {
    ctype: CmdType,
    flags: CmdFlags,
    cycle: u8,

    req: Message,
    reply: Option<Message>,

    subs: Option<Vec<Cmd>>,

    total_tracker: Option<Tracker>,

    remote_tracker: Option<Tracker>,
}

impl Command {
    fn is_done(&self) -> bool {
        self.flags & CmdFlags::DONE == CmdFlags::DONE
    }

    fn is_error(&self) -> bool {
        self.flags & CmdFlags::ERROR == CmdFlags::ERROR
    }

    pub fn can_cycle(&self) -> bool {
        self.cycle < MAX_CYCLE
    }

    pub fn add_cycle(&mut self) {
        self.cycle += 1;
    }

    pub fn set_reply(&mut self, reply: Message) {
        self.reply = Some(reply);
        self.set_done();

        let _ = self.remote_tracker.take();
    }

    pub fn set_error(&mut self, reply: Message) {
        self.set_reply(reply);
        self.flags |= CmdFlags::ERROR;
    }

    fn set_done(&mut self) {
        self.flags |= CmdFlags::DONE;
    }
}

#[derive(Default)]
pub struct FrontCodec {}

impl Decoder for FrontCodec {
    type Item = Cmd;
    type Error = AsError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match Message::parse(src).map(|x| x.map(Into::into)) {
            Ok(val) => Ok(val),
            Err(AsError::BadMessage) => {
                let cmd: Cmd = Message::raw_inline_reply().into();
                cmd.set_error(&AsError::BadMessage);
                Ok(Some(cmd))
            }
            Err(err) => Err(err),
        }
    }
}

impl Encoder<Cmd> for FrontCodec {
    type Error = AsError;
    fn encode(&mut self, item: Cmd, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let mut cmd = item.take_cmd_mut();
        if let Some(subs) = cmd.subs.as_ref().cloned() {
            for sub in subs {
                self.encode(sub, dst)?;
            }
            cmd.req.try_save_ends(dst);
        } else {
            let reply = cmd.reply.take().expect("reply must exits");
            cmd.req.save_reply(reply, dst)?;
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct BackCodec {}

impl Decoder for BackCodec {
    type Item = Message;
    type Error = AsError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        Message::parse(src)
    }
}

impl Encoder<Cmd> for BackCodec {
    type Error = AsError;
    fn encode(&mut self, item: Cmd, dst: &mut BytesMut) -> Result<(), Self::Error> {
        item.take_cmd().req.save_req(dst)
    }
}

#[test]
fn test_mc_parse_wrong_case() {
    test_mc_parse_error_in_path("../fuzz/corpus/fuzz_mc_parser/");
    test_mc_parse_error_in_path("../fuzz/artifacts/fuzz_mc_parser/");
}

#[cfg(test)]
fn test_mc_parse_error_in_path(prefix: &str) {
    use std::fs::{self, File};
    use std::io::prelude::*;
    use std::io::BufReader;

    if let Ok(dir) = fs::read_dir(prefix) {
        for entry in dir {
            let entry = entry.unwrap();
            let path = entry.path();
            println!("parsing abs_path: {:?}", path);
            let fd = File::open(path).unwrap();
            let mut buffer = BufReader::new(fd);
            let mut data = Vec::new();
            buffer.read_to_end(&mut data).unwrap();
            println!("data is {:?}", &data[..]);
            let mut src = BytesMut::from(&data[..]);
            let mut codec = FrontCodec::default();

            loop {
                let result = codec.decode(&mut src);
                match result {
                    Ok(Some(_)) => {}
                    Ok(None) => break,
                    Err(_err) => break,
                }
            }
        }
    }
}
