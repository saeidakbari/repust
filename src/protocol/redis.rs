mod cmd;
// Path src/protocol/redis/cmd.rs

mod resp;
// Path src/protocol/redis/resp.rs

use btoi::btoi;
use bytes::{Bytes, BytesMut};
use log::{debug, error, trace, warn};
use std::collections::{BTreeMap, HashSet};
use std::fmt::Debug;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::task::Waker;
use std::time::Instant;
use std::u64;
use tokio_util::codec::{Decoder, Encoder};

use crate::com::{meta, AsError};
use crate::metrics::global_error_incr;
use crate::metrics::tracker::{remote_tracker, total_tracker, Tracker};
use crate::protocol::IntoReply;
use crate::protocol::{CmdFlags, CmdType};
use crate::proxy::Request;
use crate::utils::helper::{itoa, trim_hash_tag, upper};

use resp::{Message, MessageMut, RespType};
use resp::{RESP_ERROR, RESP_INT, RESP_STRING};

pub use cmd::init_cmds as init_redis_supported_cmds;

pub const SLOTS_COUNT: usize = 16384;

const BYTES_CMD_CLUSTER: &[u8] = b"CLUSTER";
const BYTES_CMD_QUIT: &[u8] = b"QUIT";
const BYTES_SLOTS: &[u8] = b"SLOTS";
const BYTES_NODES: &[u8] = b"NODES";

#[derive(Clone, Debug)]
pub struct Cmd {
    cmd: Arc<RwLock<Command>>,
    waker: Option<Waker>,
    addr: Option<String>,
}

impl Request for Cmd {
    type Reply = Message;
    type FrontCodec = RedisHandleCodec;
    type BackCodec = RedisNodeCodec;

    fn ping_request() -> Self {
        let msg = Message::new_ping_request();
        let flags = CmdFlags::empty();
        let cmd_type = CmdType::get_cmd_type(&msg);

        let cmd = Command {
            flags,
            cmd_type,
            cycle: DEFAULT_CYCLE,
            req: msg,
            reply: None,
            subs: None,

            total_tracker: None,

            remote_tracker: None,
        };
        cmd.into_cmd()
    }

    fn auth_request(auth: &str) -> Self {
        let msg = Message::new_auth(auth);
        let flags = CmdFlags::empty();
        let cmd_type = CmdType::get_cmd_type(&msg);

        let cmd = Command {
            flags,
            cmd_type,
            cycle: DEFAULT_CYCLE,
            req: msg,
            reply: None,
            subs: None,
            total_tracker: None,
            remote_tracker: None,
        };
        cmd.into_cmd()
    }

    fn key_hash(&self, hash_tag: &[u8], hasher: fn(&[u8]) -> u64) -> u64 {
        self.take_cmd().key_hash(hash_tag, hasher)
    }

    fn subs(&self) -> Option<Vec<Self>> {
        self.take_cmd().subs.clone()
    }

    fn is_done(&self) -> bool {
        if let Some(subs) = self.subs() {
            subs.into_iter().all(|x| x.is_done())
        } else {
            self.take_cmd().is_done()
        }
    }

    fn is_error(&self) -> bool {
        self.take_cmd().is_error()
    }

    fn add_cycle(&self) {
        self.take_cmd_mut().add_cycle()
    }
    fn can_cycle(&self) -> bool {
        self.take_cmd().can_cycle()
    }

    fn valid(&self) -> bool {
        self.check_valid()
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
        self.wakeup();
    }

    fn set_error(&self, t: &AsError) {
        let reply: Message = t.into_reply();
        let mut cmd = self.take_cmd_mut();
        cmd.set_reply(reply);
        cmd.set_error();

        global_error_incr();
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
    pub fn cluster_mark_total(&self) {
        let timer = total_tracker();
        self.take_cmd_mut().total_tracker.replace(timer);
    }

    pub fn cluster_mark_remote(&self) {
        let timer = remote_tracker();
        if self.take_cmd().remote_tracker.is_none() {
            self.take_cmd_mut().remote_tracker.replace(timer);
        }
    }

    pub fn take_cmd(&self) -> RwLockReadGuard<Command> {
        self.cmd.read().unwrap()
    }

    pub fn take_cmd_mut(&self) -> RwLockWriteGuard<Command> {
        self.cmd.write().unwrap()
    }

    pub fn unset_error(&self) {
        self.take_cmd_mut().unset_error();
    }

    pub fn unset_done(&self) {
        self.take_cmd_mut().unset_done();
    }

    pub fn set_reply<T: IntoReply<Message>>(&self, reply: T) {
        self.take_cmd_mut().set_reply(reply);
    }

    pub fn set_error<T: IntoReply<Message>>(&self, reply: T) {
        self.take_cmd_mut().set_reply(reply);
        self.take_cmd_mut().set_error();
    }

    pub fn set_subs(&mut self, subs: Option<Vec<Cmd>>) {
        self.take_cmd_mut().set_subs(subs);
    }

    pub fn set_no_auth(&self) {
        self.take_cmd_mut().set_reply(AsError::NoAuth);
    }

    pub fn set_auth_wrong(&self) {
        self.take_cmd_mut().set_reply(AsError::AuthWrong);
    }

    pub fn check_valid(&self) -> bool {
        if self.take_cmd().cmd_type.is_not_support() {
            self.take_cmd_mut().set_reply(AsError::RequestNotSupport);
            return false;
        }
        if self.take_cmd().is_done() {
            return true;
        }

        if self.take_cmd().cmd_type.is_ctrl() {
            let is_quit = self
                .take_cmd()
                .req
                .nth(0)
                .map(|x| x == BYTES_CMD_QUIT)
                .unwrap_or(false);
            if is_quit {
                self.take_cmd_mut()
                    .set_reply(Message::inline_raw(Bytes::new()));
                return false;
            }

            // check if is cluster
            let is_cluster = self
                .take_cmd()
                .req
                .nth(0)
                .map(|x| x == BYTES_CMD_CLUSTER)
                .unwrap_or(false);
            if is_cluster {
                let sub_cmd = self.take_cmd().req.nth(1).map(|x| x.to_vec());
                if let Some(mut sub_cmd) = sub_cmd {
                    upper(&mut sub_cmd);
                    if sub_cmd == BYTES_SLOTS {
                        let mut data = build_cluster_slots_reply();
                        if let Ok(Some(msg)) =
                            MessageMut::parse(&mut data).map(|x| x.map(|y| y.into()))
                        {
                            let msg: Message = msg;
                            self.take_cmd_mut().set_reply(msg);
                            return false;
                        };
                    } else if sub_cmd == BYTES_NODES {
                        let mut data = build_cluster_nodes_reply();
                        if let Ok(Some(msg)) =
                            MessageMut::parse(&mut data).map(|x| x.map(|y| y.into()))
                        {
                            let msg: Message = msg;
                            self.take_cmd_mut().set_reply(msg);
                            return false;
                        };
                    }
                }
            }
            self.take_cmd_mut().set_reply(AsError::RequestNotSupport);
            return false;
        }
        // and other conditions
        true
    }

    pub fn change_info_resp(&mut self) {
        let mut cmd = self.take_cmd_mut();

        // Only for simple INFO command
        if cmd.cmd_type.is_info() {
            if let RespType::Array(_, items) = &cmd.req.resp_type {
                if items.len() == 1 {
                    if let Some(msg) = &mut cmd.reply {
                        msg.replace_info_resp();
                    }
                }
            }
        }
    }

    pub fn mk_read_all_subs(&mut self, addrs: Vec<String>) {
        let mut subs = Vec::with_capacity(addrs.len());
        for addr in addrs {
            let sub = Command {
                flags: self.take_cmd().flags(),
                cmd_type: self.take_cmd().cmd_type(),
                cycle: DEFAULT_CYCLE,
                req: self.take_cmd().req().clone(),
                reply: None,
                subs: None,
                total_tracker: None,
                remote_tracker: None,
            };

            let mut sub_cmd = sub.into_cmd();
            sub_cmd.addr = Some(addr.clone());
            subs.push(sub_cmd);
        }

        self.take_cmd_mut().set_subs(Some(subs));
    }

    pub fn get_addr(&self) -> Option<String> {
        self.addr.clone()
    }

    fn wakeup(&self) {
        if let Some(waker) = self.waker.as_ref() {
            waker.wake_by_ref();
        }
    }
}

#[derive(Debug)]
pub struct Command {
    flags: CmdFlags,
    cmd_type: CmdType,

    // Command redirect count
    cycle: u8,

    req: Message,
    reply: Option<Message>,
    subs: Option<Vec<Cmd>>,

    total_tracker: Option<Tracker>,
    remote_tracker: Option<Tracker>,
}

const BYTES_JUST_OK: &[u8] = b"+OK\r\n";
const BYTES_NULL_ARRAY: &[u8] = b"*-1\r\n";
const BYTES_ZERO_INT: &[u8] = b":0\r\n";
const BYTES_CMD_PING: &[u8] = b"PING";
const BYTES_CMD_COMMAND: &[u8] = b"COMMAND";
const BYTES_REPLY_NULL_ARRAY: &[u8] = b"*-1\r\n";
const STR_REPLY_PONG: &str = "PONG";
const BYTES_CMD_INFO_KEYSPACE: &[u8] = b"*2\r\n$4\r\nINFO\r\n$8\r\nkeyspace\r\n";

const BYTES_CRLF: &[u8] = b"\r\n";

const BYTES_ARRAY: &[u8] = b"*";
const BYTES_INTEGER: &[u8] = b":";
const BYTES_BULK_STRING: &[u8] = b"$";

const DEFAULT_CYCLE: u8 = 0;
const MAX_CYCLE: u8 = 1;

// for front end interaction
impl Command {
    pub fn into_cmd(self) -> Cmd {
        Cmd {
            cmd: Arc::new(RwLock::new(self)),
            waker: None,
            addr: None,
        }
    }

    pub fn with_addr(self, addr: String) -> Cmd {
        Cmd {
            cmd: Arc::new(RwLock::new(self)),
            waker: None,
            addr: Some(addr),
        }
    }

    pub fn parse_cmd(buf: &mut BytesMut) -> Result<Option<Cmd>, AsError> {
        let msg = MessageMut::parse(buf)?;
        trace!("msg: {:?}", msg);
        Ok(msg.map(Into::into))
    }

    pub fn reply_cmd(&self, buf: &mut BytesMut) -> Result<usize, AsError> {
        if self.cmd_type.is_mset() || self.cmd_type.is_client() {
            buf.extend_from_slice(BYTES_JUST_OK);
            Ok(BYTES_JUST_OK.len())
        } else if self.cmd_type.is_mget() {
            if let Some(subs) = self.subs.as_ref() {
                buf.extend_from_slice(BYTES_ARRAY);

                let begin = buf.len();
                let len = subs.len();

                itoa(len, buf);
                buf.extend_from_slice(BYTES_CRLF);
                for sub in subs {
                    sub.take_cmd().reply_raw(buf)?;
                }
                Ok(buf.len() - begin)
            } else {
                debug!("subs is empty");
                buf.extend_from_slice(BYTES_NULL_ARRAY);
                Ok(BYTES_NULL_ARRAY.len())
            }
        } else if self.cmd_type.is_read_all() {
            if let Some(subs) = self.subs.as_ref() {
                buf.extend_from_slice(BYTES_ARRAY);

                let begin = buf.len();
                let mut len = 0;

                for sub in subs {
                    if let Some(reply) = &sub.take_cmd().reply {
                        if let RespType::Array(_, array) = &reply.resp_type {
                            len += array.len();
                        }
                    }
                }

                itoa(len, buf);
                buf.extend_from_slice(BYTES_CRLF);

                for sub in subs {
                    sub.take_cmd().reply_inner_array(buf)?;
                }
                Ok(buf.len() - begin)
            } else {
                debug!("subs is empty");
                buf.extend_from_slice(BYTES_NULL_ARRAY);
                Ok(BYTES_NULL_ARRAY.len())
            }
        } else if self.is_info_keyspace() {
            if let Some(subs) = self.subs.as_ref() {
                buf.extend_from_slice(BYTES_BULK_STRING);

                let begin = buf.len();

                let mut keys_sum = 0;
                let mut expires_sum = 0;
                let mut avg_ttl_sum = 0;

                for sub in subs {
                    if let Some(reply) = &sub.take_cmd().reply {
                        let data = String::from_utf8_lossy(reply.data.as_ref());
                        if data.contains("# Keyspace") {
                            let idx1 = match data.find("keys=") {
                                Some(idx) => idx,
                                None => {
                                    error!("keyspace format error: {}", data);
                                    break;
                                }
                            };
                            let str1 = &data[idx1 + "keys=".len()..];
                            let idx2 = match str1.find(',') {
                                Some(idx) => idx,
                                None => {
                                    error!("keyspace format error: {}", data);
                                    break;
                                }
                            };
                            let keys: i32 = match str1[..idx2].parse() {
                                Ok(val) => val,
                                Err(_) => {
                                    error!("keyspace format error: {}", data);
                                    break;
                                }
                            };

                            let idx3 = match str1.find("expires=") {
                                Some(idx) => idx,
                                None => {
                                    error!("keyspace format error: {}", data);
                                    break;
                                }
                            };
                            let str2 = &str1[idx3 + "expires=".len()..];
                            let idx4 = match str2.find(',') {
                                Some(idx) => idx,
                                None => {
                                    error!("keyspace format error: {}", data);
                                    break;
                                }
                            };
                            let expires: i32 = match str2[..idx4].parse() {
                                Ok(val) => val,
                                Err(_) => {
                                    error!("keyspace format error: {}", data);
                                    break;
                                }
                            };

                            let idx5 = match str2.find("avg_ttl=") {
                                Some(idx) => idx,
                                None => {
                                    error!("keyspace format error: {}", data);
                                    break;
                                }
                            };
                            let str3 = &str2[idx5 + "avg_ttl=".len()..];
                            let idx6 = match str3.find('\r') {
                                Some(idx) => idx,
                                None => {
                                    error!("keyspace format error: {}", data);
                                    break;
                                }
                            };
                            let avg_ttl: i32 = match str3[..idx6].parse() {
                                Ok(val) => val,
                                Err(_) => {
                                    error!("keyspace format error: {}", data);
                                    break;
                                }
                            };

                            keys_sum += keys;
                            expires_sum += expires * keys;
                            avg_ttl_sum += avg_ttl * keys;
                        }
                    }
                }
                let keys = keys_sum;
                let expires = if keys == 0 { 0 } else { expires_sum / keys };
                let avg_ttl = if keys == 0 { 0 } else { avg_ttl_sum / keys };

                let data = format!(
                    "# Keyspace\r\ndb0:keys={},expires={},avg_ttl={}\r\n",
                    keys, expires, avg_ttl
                );

                itoa(data.len(), buf);
                buf.extend_from_slice(BYTES_CRLF);

                buf.extend_from_slice(data.as_bytes());
                buf.extend_from_slice(BYTES_CRLF);
                debug!("buf: {:?}", buf);

                Ok(buf.len() - begin)
            } else {
                debug!("subs is empty");
                buf.extend_from_slice(BYTES_NULL_ARRAY);
                Ok(BYTES_NULL_ARRAY.len())
            }
        } else if self.cmd_type.is_scan() {
            if let Some(subs) = self.subs.as_ref() {
                buf.extend_from_slice(BYTES_ARRAY);
                itoa(2, buf);
                buf.extend_from_slice(BYTES_CRLF);

                let begin = buf.len();
                let next_idx = 0;
                let mut len = 0;

                for sub in subs {
                    if let Some(reply) = &sub.take_cmd().reply {
                        if let RespType::Array(_, array) = &reply.resp_type {
                            if array.len() == 2 {
                                let inner_arr = &array[1];
                                if let RespType::Array(_, array) = inner_arr {
                                    len += array.len();
                                }
                            }
                        }
                    }
                }

                buf.extend_from_slice(BYTES_BULK_STRING);
                itoa(next_idx.to_string().len(), buf);

                buf.extend_from_slice(BYTES_CRLF);
                itoa(next_idx, buf);

                buf.extend_from_slice(BYTES_CRLF);
                buf.extend_from_slice(BYTES_ARRAY);
                itoa(len, buf);

                buf.extend_from_slice(BYTES_CRLF);

                for sub in subs {
                    sub.take_cmd().reply_inner_inner_array(buf)?;
                }
                Ok(buf.len() - begin)
            } else {
                debug!("subs is empty");
                buf.extend_from_slice(BYTES_NULL_ARRAY);
                Ok(BYTES_NULL_ARRAY.len())
            }
        } else if self.cmd_type.is_del()
            || self.cmd_type.is_exists()
            || self.cmd_type.is_count_all()
        {
            if let Some(subs) = self.subs.as_ref() {
                let begin = buf.len();
                buf.extend_from_slice(BYTES_INTEGER);

                let mut total = 0usize;
                for sub in subs {
                    if let Some(Some(data)) = sub.take_cmd().reply.as_ref().map(|x| x.nth(0)) {
                        total += btoi::<usize>(data).unwrap_or(0);
                    }
                }

                itoa(total, buf);
                buf.extend_from_slice(BYTES_CRLF);
                Ok(buf.len() - begin)
            } else {
                buf.extend_from_slice(BYTES_ZERO_INT);
                Ok(BYTES_ZERO_INT.len())
            }
        } else {
            self.reply_raw(buf)
        }
    }

    fn reply_raw(&self, buf: &mut BytesMut) -> Result<usize, AsError> {
        self.reply
            .as_ref()
            .map(|x| x.save(buf))
            .ok_or_else(|| AsError::BadReply)
    }

    fn reply_inner_array(&self, buf: &mut BytesMut) -> Result<usize, AsError> {
        let mut size = 0usize;
        if let Some(reply) = &self.reply {
            if let RespType::Array(_, subs) = &reply.resp_type {
                for sub in subs {
                    size += reply.save_by_resp_type(sub, buf);
                }
            }
        }
        Ok(size)
    }

    fn reply_inner_inner_array(&self, buf: &mut BytesMut) -> Result<usize, AsError> {
        let mut size = 0usize;
        if let Some(reply) = &self.reply {
            if let RespType::Array(_, subs) = &reply.resp_type {
                if subs.len() == 2 {
                    let inner_arr = &subs[1];
                    if let RespType::Array(_, subs) = inner_arr {
                        for sub in subs {
                            size += reply.save_by_resp_type(sub, buf);
                        }
                    }
                }
            }
        }
        Ok(size)
    }
}

const BYTES_ASK: &[u8] = b"*1\r\n$3\r\nASK\r\n";
const BYTES_GET: &[u8] = b"$3\r\nGET\r\n";
const BYTES_LEN2_HEAD: &[u8] = b"*2\r\n";
const BYTES_LEN3_HEAD: &[u8] = b"*3\r\n";

// for back end interaction
impl Command {
    /// save redis Command into given BytesMut
    pub fn send_req(&self, buf: &mut BytesMut) -> Result<(), AsError> {
        if self.is_ask() {
            buf.extend_from_slice(BYTES_ASK);
        }

        if self.cmd_type.is_exists() || self.cmd_type.is_del() {
            buf.extend_from_slice(BYTES_LEN2_HEAD);
            if let RespType::Array(_, arrays) = &self.req.resp_type {
                for resp_type in arrays {
                    self.req.save_by_resp_type(resp_type, buf);
                }
            }
            return Ok(());
        } else if self.cmd_type.is_mset() {
            buf.extend_from_slice(BYTES_LEN3_HEAD);
            if let RespType::Array(_, arrays) = &self.req.resp_type {
                for resp_type in arrays {
                    self.req.save_by_resp_type(resp_type, buf);
                }
            }
            return Ok(());
        } else if self.cmd_type.is_mget() {
            buf.extend_from_slice(BYTES_LEN2_HEAD);
            buf.extend_from_slice(BYTES_GET);

            if let RespType::Array(_, arrays) = &self.req.resp_type {
                for resp_type in &arrays[1..] {
                    self.req.save_by_resp_type(resp_type, buf);
                }
            }
            return Ok(());
        }
        self.req.save(buf);
        Ok(())
    }
}

impl Command {
    pub fn key_hash<T>(&self, hash_tag: &[u8], method: T) -> u64
    where
        T: Fn(&[u8]) -> u64,
    {
        let pos = self.key_pos();

        if let Some(key_data) = self.req.nth(pos) {
            method(trim_hash_tag(key_data, hash_tag)) as u64
        } else {
            u64::MAX
        }
    }

    #[inline(always)]
    fn key_pos(&self) -> usize {
        if self.cmd_type.is_eval() {
            return KEY_EVAL_POS;
        } else if self.cmd_type.is_info() || self.cmd_type.is_command() {
            return COMMAND_POS;
        } else if self.cmd_type.is_memory() {
            return KEY_MEMORY_POS;
        }
        KEY_RAW_POS
    }

    pub fn subs(&self) -> Option<Vec<Cmd>> {
        self.subs.as_ref().cloned()
    }

    pub fn set_subs(&mut self, subs: Option<Vec<Cmd>>) {
        self.subs = subs;
    }

    pub fn is_done(&self) -> bool {
        if self.subs.is_some() {
            return self
                .subs
                .as_ref()
                .map(|x| x.iter().all(|y| y.take_cmd().is_done()))
                .unwrap_or(false);
        }
        self.flags & CmdFlags::DONE == CmdFlags::DONE
    }

    fn set_reply<T: IntoReply<Message>>(&mut self, reply: T) {
        self.reply = Some(reply.into_reply());
        self.set_done();

        let _ = self.remote_tracker.take();
    }

    fn set_done(&mut self) {
        self.flags |= CmdFlags::DONE;
    }

    fn unset_done(&mut self) {
        self.flags &= !CmdFlags::DONE;
    }

    fn unset_error(&mut self) {
        self.flags &= !CmdFlags::ERROR;
    }

    fn set_error(&mut self) {
        self.flags |= CmdFlags::ERROR;
    }

    pub fn cycle(&self) -> u8 {
        self.cycle
    }

    pub fn can_cycle(&self) -> bool {
        self.cycle < MAX_CYCLE
    }

    pub fn add_cycle(&mut self) {
        self.cycle += 1;
    }

    pub fn is_ask(&self) -> bool {
        self.flags & CmdFlags::ASK == CmdFlags::ASK
    }

    pub fn unset_ask(&mut self) {
        self.flags &= !CmdFlags::ASK;
    }

    pub fn set_ask(&mut self) {
        self.flags |= CmdFlags::ASK;
    }

    pub fn is_moved(&mut self) -> bool {
        self.flags & CmdFlags::MOVED == CmdFlags::MOVED
    }

    pub fn set_moved(&mut self) {
        self.flags |= CmdFlags::MOVED;
    }

    pub fn unset_moved(&mut self) {
        self.flags &= !CmdFlags::MOVED;
    }

    pub fn is_error(&self) -> bool {
        if self.subs.is_some() {
            return self
                .subs
                .as_ref()
                .map(|x| x.iter().any(|y| y.take_cmd().is_error()))
                .unwrap_or(false);
        }
        self.flags & CmdFlags::ERROR == CmdFlags::ERROR
    }

    pub fn is_read(&self) -> bool {
        self.cmd_type.is_read()
    }

    pub fn is_read_all(&self) -> bool {
        self.cmd_type.is_read_all()
    }

    pub fn is_count_all(&self) -> bool {
        self.cmd_type.is_count_all()
    }

    pub fn is_scan(&self) -> bool {
        self.cmd_type.is_scan()
    }

    pub fn is_info_keyspace(&self) -> bool {
        self.cmd_type.is_info() && self.req.data == BYTES_CMD_INFO_KEYSPACE
    }

    pub fn flags(&self) -> CmdFlags {
        self.flags
    }

    pub fn cmd_type(&self) -> CmdType {
        self.cmd_type
    }

    pub fn req(&self) -> &Message {
        &self.req
    }
}

impl Command {
    fn mk_mset(flags: CmdFlags, ctype: CmdType, msg: Message) -> Cmd {
        let Message { resp_type, data } = msg.clone();
        if let RespType::Array(head, array) = resp_type {
            let array_len = array.len();

            if array_len > MAX_KEY_COUNT {
                // TODO: forbidden large request
                unimplemented!();
            }

            let cmd_count = array_len / 2;
            let mut subs = Vec::with_capacity(cmd_count / 2);

            for chunk in array[1..].chunks(2) {
                let key = chunk[0].clone();
                let val = chunk[1].clone();

                let sub = Message {
                    resp_type: RespType::Array(head, vec![array[0].clone(), key, val]),
                    data: data.clone(),
                };

                let sub_cmd = Command {
                    flags,
                    cmd_type: ctype,
                    cycle: DEFAULT_CYCLE,
                    req: sub,
                    reply: None,
                    subs: None,
                    total_tracker: None,
                    remote_tracker: None,
                };

                subs.push(sub_cmd.into_cmd());
            }

            let command = Command {
                flags,
                cmd_type: ctype,
                cycle: DEFAULT_CYCLE,
                subs: Some(subs),
                req: msg,
                reply: None,
                total_tracker: None,
                remote_tracker: None,
            };
            command.into_cmd()
        } else {
            let cmd = Command {
                flags,
                cycle: DEFAULT_CYCLE,
                cmd_type: ctype,
                req: msg,
                reply: None,
                subs: None,
                total_tracker: None,
                remote_tracker: None,
            };
            let cmd = cmd.into_cmd();
            cmd.set_reply(&AsError::RequestInlineWithMultiKeys);
            cmd
        }
    }

    fn mk_subs(flags: CmdFlags, cmd_type: CmdType, msg: Message) -> Cmd {
        let Message { resp_type, data } = msg.clone();
        if let RespType::Array(head, array) = resp_type {
            let array_len = array.len();
            // TODO: maybe checking for the huge response size would be a good idea

            let mut subs = Vec::with_capacity(array_len - 1);
            for key in &array[1..] {
                let sub = Message {
                    resp_type: RespType::Array(head, vec![array[0].clone(), key.clone()]),
                    data: data.clone(),
                };

                let sub_cmd = Command {
                    flags,
                    cmd_type,
                    cycle: DEFAULT_CYCLE,
                    req: sub,
                    reply: None,
                    subs: None,
                    total_tracker: None,
                    remote_tracker: None,
                };

                subs.push(sub_cmd.into_cmd());
            }

            let cmd = Command {
                flags,
                cycle: DEFAULT_CYCLE,
                cmd_type,
                req: msg,
                reply: None,
                subs: Some(subs),
                total_tracker: None,
                remote_tracker: None,
            };
            cmd.into_cmd()
        } else {
            let cmd = Command {
                flags,
                cycle: DEFAULT_CYCLE,
                cmd_type,
                req: msg,
                reply: None,
                subs: None,
                total_tracker: None,
                remote_tracker: None,
            };
            let cmd = cmd.into_cmd();
            cmd.set_reply(&AsError::RequestInlineWithMultiKeys);
            cmd
        }
    }
}

const COMMAND_POS: usize = 0;
const KEY_EVAL_POS: usize = 3;
const KEY_RAW_POS: usize = 1;
const KEY_MEMORY_POS: usize = 2;
const MAX_KEY_COUNT: usize = 10000;

impl From<MessageMut> for Cmd {
    fn from(mut msg_mut: MessageMut) -> Cmd {
        // upper the given command
        if let Some(data) = msg_mut.nth_mut(COMMAND_POS) {
            upper(data);
        } else {
            let msg = msg_mut.into();
            let ctype = CmdType::NotSupport;
            let flags = CmdFlags::empty();
            let command = Command {
                flags,
                cmd_type: ctype,
                cycle: DEFAULT_CYCLE,
                req: msg,
                reply: None,
                subs: None,
                total_tracker: None,
                remote_tracker: None,
            };
            let cmd: Cmd = command.into_cmd();
            cmd.set_reply(AsError::RequestNotSupport);
            return cmd;
        }

        let msg = msg_mut.into();
        let ctype = CmdType::get_cmd_type(&msg);
        let flags = CmdFlags::empty();

        if ctype.is_exists() || ctype.is_del() || ctype.is_mget() {
            return Command::mk_subs(flags, ctype, msg);
        } else if ctype.is_mset() {
            return Command::mk_mset(flags, ctype, msg);
        }

        let mut cmd = Command {
            flags,
            cmd_type: ctype,
            cycle: DEFAULT_CYCLE,
            req: msg.clone(),
            reply: None,
            subs: None,
            total_tracker: None,
            remote_tracker: None,
        };
        if ctype.is_ctrl() {
            if let Some(data) = msg.nth(COMMAND_POS) {
                if data == BYTES_CMD_PING {
                    cmd.set_reply(STR_REPLY_PONG);
                    cmd.unset_error();
                } else if data == BYTES_CMD_COMMAND {
                    cmd.set_reply(BYTES_REPLY_NULL_ARRAY);
                    cmd.unset_error();
                } else {
                    // unsupported commands
                    trace!("unsupported commands");
                }
            }
        }
        cmd.into_cmd()
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RedisHandleCodec {}

impl Decoder for RedisHandleCodec {
    type Item = Cmd;
    type Error = AsError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        Command::parse_cmd(src)
    }
}

impl Encoder<Cmd> for RedisHandleCodec {
    type Error = AsError;
    fn encode(&mut self, item: Cmd, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let _ = item.take_cmd().reply_cmd(dst)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RedisNodeCodec {}

impl Decoder for RedisNodeCodec {
    type Item = Message;
    type Error = AsError;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let reply = MessageMut::parse(src)?;
        Ok(reply.map(Into::into))
    }
}

impl Encoder<Cmd> for RedisNodeCodec {
    type Error = AsError;
    fn encode(&mut self, item: Cmd, dst: &mut BytesMut) -> Result<(), Self::Error> {
        item.take_cmd().send_req(dst)
    }
}

pub fn new_read_only_cmd() -> Cmd {
    let msg = Message::new_read_only();
    let flags = CmdFlags::empty();
    let ctype = CmdType::get_cmd_type(&msg);

    let cmd = Command {
        flags,
        cmd_type: ctype,
        cycle: DEFAULT_CYCLE,
        req: msg,
        reply: None,
        subs: None,
        total_tracker: None,
        remote_tracker: None,
    };
    cmd.into_cmd()
}

pub fn new_cluster_slots_cmd() -> Cmd {
    let msg = Message::new_cluster_slots();
    let flags = CmdFlags::empty();
    let cmd_type = CmdType::get_cmd_type(&msg);

    let cmd = Command {
        flags,
        cmd_type,
        cycle: DEFAULT_CYCLE,
        req: msg,
        reply: None,
        subs: None,
        total_tracker: None,
        remote_tracker: None,
    };
    cmd.into_cmd()
}

pub fn new_auth_cmd(auth: &str) -> Cmd {
    let msg = Message::new_auth(auth);
    let flags = CmdFlags::empty();
    let ctype = CmdType::get_cmd_type(&msg);

    let cmd = Command {
        flags,
        cmd_type: ctype,
        cycle: DEFAULT_CYCLE,
        req: msg,
        reply: None,
        subs: None,
        total_tracker: None,
        remote_tracker: None,
    };
    cmd.into_cmd()
}

pub type ReplicaLayout = (Vec<String>, Vec<Vec<String>>);

pub fn slots_reply_to_replicas(cmd: Cmd) -> Result<Option<ReplicaLayout>, AsError> {
    let msg = cmd
        .take_cmd_mut()
        .reply
        .take()
        .expect("reply must be non-empty");
    match msg.resp_type {
        RespType::Array(_, ref subs) => {
            let mut mrepusts = BTreeMap::<usize, String>::new();
            let mut replicas = BTreeMap::<usize, HashSet<String>>::new();

            let get_data = |index: usize, each: &[RespType]| -> Result<&[u8], AsError> {
                let frg = msg
                    .get_range(each.get(index))
                    .ok_or(AsError::WrongClusterSlotsReplySlot)?;
                Ok(msg.get_data_of_range(frg))
            };

            let get_number = |index: usize, each: &[RespType]| -> Result<usize, AsError> {
                let data = get_data(index, each)?;
                let num = btoi::<usize>(data)?;
                Ok(num)
            };

            let get_addr = |each: &RespType| -> Result<String, AsError> {
                if let RespType::Array(_, ref inner) = each {
                    let port = get_number(1, inner)?;
                    let addr = String::from_utf8_lossy(get_data(0, inner)?);
                    Ok(format!("{}:{}", addr, port))
                } else {
                    Err(AsError::WrongClusterSlotsReplyType)
                }
            };

            for sub in subs {
                if let RespType::Array(_, ref subs) = sub {
                    let begin = get_number(0, subs)?;
                    let end = get_number(1, subs)?;
                    let mrepust =
                        get_addr(subs.get(2).ok_or(AsError::WrongClusterSlotsReplyType)?)?;
                    for i in begin..=end {
                        mrepusts.insert(i, mrepust.clone());
                    }
                    for i in begin..=end {
                        replicas.insert(i, HashSet::new());
                    }
                    if subs.len() > 3 {
                        let mut replica_set = HashSet::new();
                        for resp in subs.iter().skip(3) {
                            let replica = get_addr(resp)?;
                            replica_set.insert(replica);
                        }
                        for i in begin..=end {
                            replicas.insert(i, replica_set.clone());
                        }
                    }
                } else {
                    return Err(AsError::WrongClusterSlotsReplyType);
                }
            }
            if mrepusts.len() != SLOTS_COUNT {
                warn!("slots is not full covered but ignore it");
            }
            let mrepust_list = mrepusts.into_values().collect();
            let replicas_list = replicas
                .into_values()
                .map(|v| v.into_iter().collect())
                .collect();
            Ok(Some((mrepust_list, replicas_list)))
        }
        _ => Err(AsError::WrongClusterSlotsReplyType),
    }
}

impl From<AsError> for Message {
    fn from(err: AsError) -> Message {
        err.into_reply()
    }
}

impl IntoReply<Message> for AsError {
    fn into_reply(self) -> Message {
        let value = format!("{}", self);
        Message::plain(value.as_bytes().to_owned(), RESP_ERROR)
    }
}

impl<'a> IntoReply<Message> for &'a AsError {
    fn into_reply(self) -> Message {
        let value = format!("{}", self);
        Message::plain(value.as_bytes().to_owned(), RESP_ERROR)
    }
}

impl<'a> IntoReply<Message> for &'a str {
    fn into_reply(self) -> Message {
        let value = self.to_string();
        Message::plain(value.as_bytes().to_owned(), RESP_STRING)
    }
}

impl<'a> IntoReply<Message> for &'a [u8] {
    fn into_reply(self) -> Message {
        let bytes = Bytes::from(self.to_owned());
        Message::inline_raw(bytes)
    }
}

impl<'a> IntoReply<Message> for &'a usize {
    fn into_reply(self) -> Message {
        let value = format!("{}", self);
        Message::plain(value.as_bytes().to_owned(), RESP_INT)
    }
}

impl IntoReply<Message> for usize {
    fn into_reply(self) -> Message {
        let value = format!("{}", self);
        Message::plain(value.as_bytes().to_owned(), RESP_INT)
    }
}

fn build_cluster_nodes_reply() -> BytesMut {
    let port = meta::get_port();
    let ip = meta::get_ip();
    let reply = format!("0000000000000000000000000000000000000001 {ip}:{port} mrepust,myself - 0 0 1 connected 0-5460\n0000000000000000000000000000000000000002 {ip}:{port} mrepust - 0 0 2 connected 5461-10922\n0000000000000000000000000000000000000003 {ip}:{port} mrepust - 0 0 3 connected 10923-16383\n", ip = ip, port = port);
    let reply = format!("${}\r\n{}\r\n", reply.len(), reply);
    let mut data = BytesMut::new();
    data.extend_from_slice(reply.as_bytes());
    data
}

fn build_cluster_slots_reply() -> BytesMut {
    let port = meta::get_port();
    let ip = meta::get_ip();
    let reply = format!("*3\r\n*3\r\n:0\r\n:5460\r\n*3\r\n${iplen}\r\n{ip}\r\n:{port}\r\n$40\r\n0000000000000000000000000000000000000001\r\n*3\r\n:5461\r\n:10922\r\n*3\r\n${iplen}\r\n{ip}\r\n:{port}\r\n$40\r\n0000000000000000000000000000000000000002\r\n*3\r\n:10923\r\n:16383\r\n*3\r\n${iplen}\r\n{ip}\r\n:{port}\r\n$40\r\n0000000000000000000000000000000000000003\r\n", iplen=ip.len(), ip = ip, port = port);
    let mut data = BytesMut::new();
    data.extend_from_slice(reply.as_bytes());
    data
}

#[test]
fn test_redis_parse_wrong_case() {
    use std::fs::{self, File};
    use std::io::prelude::*;
    use std::io::BufReader;

    let prefix = "../fuzz/corpus/fuzz_redis_parser/";

    if let Ok(dir) = fs::read_dir(prefix) {
        for entry in dir {
            let entry = entry.unwrap();
            let path = entry.path();
            println!("parsing abs_path: {:?}", path);
            let fd = File::open(path).unwrap();
            let mut buffer = BufReader::new(fd);
            let mut data = Vec::new();
            buffer.read_to_end(&mut data).unwrap();
            let mut src = BytesMut::from(&data[..]);

            loop {
                let result = Command::parse_cmd(&mut src);
                match result {
                    Ok(Some(_)) => {}
                    Ok(None) => break,
                    Err(_err) => break,
                }
            }
        }
    }
}
