use std::collections::HashMap;
use std::sync::OnceLock;

use crate::protocol::redis::resp::Message;
use crate::protocol::CmdType;

// TODO: consider to std::sync::LazyLock when the API has been finalized
static CMD_HASHMAP: OnceLock<HashMap<&[u8], CmdType>> = OnceLock::new();

pub fn init_cmds() {
    let mut cmds_hashmap: HashMap<&[u8], CmdType> = HashMap::new();

    // special commands
    cmds_hashmap.insert(&b"DEL"[..], CmdType::Del);
    cmds_hashmap.insert(&b"UNLINK"[..], CmdType::Del);
    cmds_hashmap.insert(&b"DUMP"[..], CmdType::Read);
    cmds_hashmap.insert(&b"EXISTS"[..], CmdType::Exists);
    cmds_hashmap.insert(&b"EXPIRE"[..], CmdType::Write);
    cmds_hashmap.insert(&b"EXPIREAT"[..], CmdType::Write);
    cmds_hashmap.insert(&b"KEYS"[..], CmdType::ReadAll);
    cmds_hashmap.insert(&b"DBSIZE"[..], CmdType::CountAll);
    cmds_hashmap.insert(&b"MIGRATE"[..], CmdType::NotSupport);
    cmds_hashmap.insert(&b"MOVE"[..], CmdType::NotSupport);
    cmds_hashmap.insert(&b"OBJECT"[..], CmdType::NotSupport);
    cmds_hashmap.insert(&b"PERSIST"[..], CmdType::Write);
    cmds_hashmap.insert(&b"PEXPIRE"[..], CmdType::Write);
    cmds_hashmap.insert(&b"PEXPIREAT"[..], CmdType::Write);
    cmds_hashmap.insert(&b"PTTL"[..], CmdType::Read);
    cmds_hashmap.insert(&b"RANDOMKEY"[..], CmdType::NotSupport);
    cmds_hashmap.insert(&b"RENAME"[..], CmdType::NotSupport);
    cmds_hashmap.insert(&b"RENAMENX"[..], CmdType::NotSupport);
    cmds_hashmap.insert(&b"RESTORE"[..], CmdType::Write);
    cmds_hashmap.insert(&b"SCAN"[..], CmdType::Scan);
    cmds_hashmap.insert(&b"SORT"[..], CmdType::Write);
    cmds_hashmap.insert(&b"TTL"[..], CmdType::Read);
    cmds_hashmap.insert(&b"TYPE"[..], CmdType::Read);
    cmds_hashmap.insert(&b"WAIT"[..], CmdType::NotSupport);
    cmds_hashmap.insert(&b"COMMAND"[..], CmdType::Command);
    cmds_hashmap.insert(&b"CLIENT"[..], CmdType::Client);
    cmds_hashmap.insert(&b"MODULE"[..], CmdType::Module);
    cmds_hashmap.insert(&b"MEMORY"[..], CmdType::Memory);

    // string key
    cmds_hashmap.insert(&b"APPEND"[..], CmdType::Write);
    cmds_hashmap.insert(&b"BITCOUNT"[..], CmdType::Read);
    cmds_hashmap.insert(&b"BITOP"[..], CmdType::NotSupport);
    cmds_hashmap.insert(&b"BITPOS"[..], CmdType::Read);
    cmds_hashmap.insert(&b"DECR"[..], CmdType::Write);
    cmds_hashmap.insert(&b"DECRBY"[..], CmdType::Write);
    cmds_hashmap.insert(&b"GET"[..], CmdType::Read);
    cmds_hashmap.insert(&b"GETBIT"[..], CmdType::Read);
    cmds_hashmap.insert(&b"GETRANGE"[..], CmdType::Read);
    cmds_hashmap.insert(&b"GETSET"[..], CmdType::Write);
    cmds_hashmap.insert(&b"INCR"[..], CmdType::Write);
    cmds_hashmap.insert(&b"INCRBY"[..], CmdType::Write);
    cmds_hashmap.insert(&b"INCRBYFLOAT"[..], CmdType::Write);
    cmds_hashmap.insert(&b"MGET"[..], CmdType::MGet);
    cmds_hashmap.insert(&b"MSET"[..], CmdType::MSet);
    cmds_hashmap.insert(&b"MSETNX"[..], CmdType::NotSupport);
    cmds_hashmap.insert(&b"PSETEX"[..], CmdType::Write);
    cmds_hashmap.insert(&b"SET"[..], CmdType::Write);
    cmds_hashmap.insert(&b"SETBIT"[..], CmdType::Write);
    cmds_hashmap.insert(&b"SETEX"[..], CmdType::Write);
    cmds_hashmap.insert(&b"SETNX"[..], CmdType::Write);
    cmds_hashmap.insert(&b"SETRANGE"[..], CmdType::Write);
    cmds_hashmap.insert(&b"BITFIELD"[..], CmdType::Write);
    cmds_hashmap.insert(&b"STRLEN"[..], CmdType::Read);
    cmds_hashmap.insert(&b"SUBSTR"[..], CmdType::Read);

    // hash type
    cmds_hashmap.insert(&b"HDEL"[..], CmdType::Write);
    cmds_hashmap.insert(&b"HEXISTS"[..], CmdType::Read);
    cmds_hashmap.insert(&b"HGET"[..], CmdType::Read);
    cmds_hashmap.insert(&b"HGETALL"[..], CmdType::Read);
    cmds_hashmap.insert(&b"HINCRBY"[..], CmdType::Write);
    cmds_hashmap.insert(&b"HINCRBYFLOAT"[..], CmdType::Write);
    cmds_hashmap.insert(&b"HKEYS"[..], CmdType::Read);
    cmds_hashmap.insert(&b"HLEN"[..], CmdType::Read);
    cmds_hashmap.insert(&b"HMGET"[..], CmdType::Read);
    cmds_hashmap.insert(&b"HMSET"[..], CmdType::Write);
    cmds_hashmap.insert(&b"HSET"[..], CmdType::Write);
    cmds_hashmap.insert(&b"HSETNX"[..], CmdType::Write);
    cmds_hashmap.insert(&b"HSTRLEN"[..], CmdType::Read);
    cmds_hashmap.insert(&b"HVALS"[..], CmdType::Read);
    cmds_hashmap.insert(&b"HSCAN"[..], CmdType::Read);

    // list type
    cmds_hashmap.insert(&b"BLPOP"[..], CmdType::NotSupport);
    cmds_hashmap.insert(&b"BRPOP"[..], CmdType::NotSupport);
    cmds_hashmap.insert(&b"BRPOPLPUSH"[..], CmdType::NotSupport);
    cmds_hashmap.insert(&b"LINDEX"[..], CmdType::Read);
    cmds_hashmap.insert(&b"LINSERT"[..], CmdType::Write);
    cmds_hashmap.insert(&b"LLEN"[..], CmdType::Read);
    cmds_hashmap.insert(&b"LPOP"[..], CmdType::Write);
    cmds_hashmap.insert(&b"LPUSH"[..], CmdType::Write);
    cmds_hashmap.insert(&b"LPUSHX"[..], CmdType::Write);
    cmds_hashmap.insert(&b"LRANGE"[..], CmdType::Read);
    cmds_hashmap.insert(&b"LREM"[..], CmdType::Write);
    cmds_hashmap.insert(&b"LSET"[..], CmdType::Write);
    cmds_hashmap.insert(&b"LTRIM"[..], CmdType::Write);
    cmds_hashmap.insert(&b"RPOP"[..], CmdType::Write);
    cmds_hashmap.insert(&b"RPOPLPUSH"[..], CmdType::Write);
    cmds_hashmap.insert(&b"RPUSH"[..], CmdType::Write);
    cmds_hashmap.insert(&b"RPUSHX"[..], CmdType::Write);

    // set type
    cmds_hashmap.insert(&b"SADD"[..], CmdType::Write);
    cmds_hashmap.insert(&b"SCARD"[..], CmdType::Read);
    cmds_hashmap.insert(&b"SDIFF"[..], CmdType::Read);
    cmds_hashmap.insert(&b"SDIFFSTORE"[..], CmdType::Write);
    cmds_hashmap.insert(&b"SINTER"[..], CmdType::Read);
    cmds_hashmap.insert(&b"SINTERSTORE"[..], CmdType::Write);
    cmds_hashmap.insert(&b"SISMEMBER"[..], CmdType::Read);
    cmds_hashmap.insert(&b"SMEMBERS"[..], CmdType::Read);
    cmds_hashmap.insert(&b"SMISMEMBER"[..], CmdType::Read);
    cmds_hashmap.insert(&b"SMOVE"[..], CmdType::Write);
    cmds_hashmap.insert(&b"SPOP"[..], CmdType::Write);
    cmds_hashmap.insert(&b"SRANDMEMBER"[..], CmdType::Read);
    cmds_hashmap.insert(&b"SREM"[..], CmdType::Write);
    cmds_hashmap.insert(&b"SUNION"[..], CmdType::Read);
    cmds_hashmap.insert(&b"SUNIONSTORE"[..], CmdType::Write);
    cmds_hashmap.insert(&b"SSCAN"[..], CmdType::Read);

    // zset type
    cmds_hashmap.insert(&b"ZADD"[..], CmdType::Write);
    cmds_hashmap.insert(&b"ZCARD"[..], CmdType::Read);
    cmds_hashmap.insert(&b"ZCOUNT"[..], CmdType::Read);
    cmds_hashmap.insert(&b"ZINCRBY"[..], CmdType::Write);
    cmds_hashmap.insert(&b"ZINTERSTORE"[..], CmdType::Write);
    cmds_hashmap.insert(&b"ZLEXCOUNT"[..], CmdType::Read);
    cmds_hashmap.insert(&b"ZRANGE"[..], CmdType::Read);
    cmds_hashmap.insert(&b"ZRANGEBYLEX"[..], CmdType::Read);
    cmds_hashmap.insert(&b"ZRANGEBYSCORE"[..], CmdType::Read);
    cmds_hashmap.insert(&b"ZRANK"[..], CmdType::Read);
    cmds_hashmap.insert(&b"ZREM"[..], CmdType::Write);
    cmds_hashmap.insert(&b"ZREMRANGEBYLEX"[..], CmdType::Write);
    cmds_hashmap.insert(&b"ZREMRANGEBYRANK"[..], CmdType::Write);
    cmds_hashmap.insert(&b"ZREMRANGEBYSCORE"[..], CmdType::Write);
    cmds_hashmap.insert(&b"ZREVRANGE"[..], CmdType::Read);
    cmds_hashmap.insert(&b"ZREVRANGEBYLEX"[..], CmdType::Read);
    cmds_hashmap.insert(&b"ZREVRANGEBYSCORE"[..], CmdType::Read);
    cmds_hashmap.insert(&b"ZREVRANK"[..], CmdType::Read);
    cmds_hashmap.insert(&b"ZSCORE"[..], CmdType::Read);
    cmds_hashmap.insert(&b"ZUNIONSTORE"[..], CmdType::Write);
    cmds_hashmap.insert(&b"ZSCAN"[..], CmdType::Read);

    // hyper log type
    cmds_hashmap.insert(&b"PFADD"[..], CmdType::Write);
    cmds_hashmap.insert(&b"PFCOUNT"[..], CmdType::Read);
    cmds_hashmap.insert(&b"PFMERGE"[..], CmdType::Write);

    // geo
    cmds_hashmap.insert(&b"GEOADD"[..], CmdType::Write);
    cmds_hashmap.insert(&b"GEODIST"[..], CmdType::Read);
    cmds_hashmap.insert(&b"GEOHASH"[..], CmdType::Read);
    cmds_hashmap.insert(&b"GEOPOS"[..], CmdType::Write);
    cmds_hashmap.insert(&b"GEORADIUS"[..], CmdType::Write);
    cmds_hashmap.insert(&b"GEORADIUSBYMEMBER"[..], CmdType::Write);

    // eval type
    cmds_hashmap.insert(&b"EVAL"[..], CmdType::Eval);
    cmds_hashmap.insert(&b"EVALSHA"[..], CmdType::NotSupport);

    // ctrl type
    cmds_hashmap.insert(&b"AUTH"[..], CmdType::Auth);
    cmds_hashmap.insert(&b"ECHO"[..], CmdType::Ctrl);
    cmds_hashmap.insert(&b"PING"[..], CmdType::Ctrl);
    cmds_hashmap.insert(&b"INFO"[..], CmdType::Info);
    cmds_hashmap.insert(&b"PROXY"[..], CmdType::NotSupport);
    cmds_hashmap.insert(&b"SLOWLOG"[..], CmdType::NotSupport);
    cmds_hashmap.insert(&b"QUIT"[..], CmdType::Ctrl);
    cmds_hashmap.insert(&b"SELECT"[..], CmdType::NotSupport);
    cmds_hashmap.insert(&b"TIME"[..], CmdType::NotSupport);
    cmds_hashmap.insert(&b"CONFIG"[..], CmdType::NotSupport);
    cmds_hashmap.insert(&b"CLUSTER"[..], CmdType::Ctrl);
    cmds_hashmap.insert(&b"READONLY"[..], CmdType::Ctrl);

    // bloom filter type
    cmds_hashmap.insert(&b"BF.ADD"[..], CmdType::Write);
    cmds_hashmap.insert(&b"BF.EXISTS"[..], CmdType::Read);
    cmds_hashmap.insert(&b"BF.INFO"[..], CmdType::Read);
    cmds_hashmap.insert(&b"BF.INSERT"[..], CmdType::Write);
    cmds_hashmap.insert(&b"BF.LOADCHUNK"[..], CmdType::NotSupport);
    cmds_hashmap.insert(&b"BF.MADD"[..], CmdType::Write);
    cmds_hashmap.insert(&b"BF.MEXISTS"[..], CmdType::Read);
    cmds_hashmap.insert(&b"BF.RESERVE"[..], CmdType::Write);
    cmds_hashmap.insert(&b"BF.SCANDUMP"[..], CmdType::NotSupport);

    // Cuckoo Filter commands.
    cmds_hashmap.insert(&b"CF.ADD"[..], CmdType::Write);
    cmds_hashmap.insert(&b"CF.ADDNX"[..], CmdType::Write);
    cmds_hashmap.insert(&b"CF.COUNT"[..], CmdType::Read);
    cmds_hashmap.insert(&b"CF.DEL"[..], CmdType::Write);
    cmds_hashmap.insert(&b"CF.EXISTS"[..], CmdType::Read);
    cmds_hashmap.insert(&b"CF.INFO"[..], CmdType::Read);
    cmds_hashmap.insert(&b"CF.INSERT"[..], CmdType::Write);
    cmds_hashmap.insert(&b"CF.INSERTNX"[..], CmdType::Write);
    cmds_hashmap.insert(&b"CF.LOADCHUNK"[..], CmdType::NotSupport);
    cmds_hashmap.insert(&b"CF.MEXISTS"[..], CmdType::Read);
    cmds_hashmap.insert(&b"CF.RESERVE"[..], CmdType::Write);
    cmds_hashmap.insert(&b"CF.SCANDUMP"[..], CmdType::NotSupport);

    // Count-Min Sketch commands.
    cmds_hashmap.insert(&b"CMS.INCRBY"[..], CmdType::Write);
    cmds_hashmap.insert(&b"CMS.INFO"[..], CmdType::Read);
    cmds_hashmap.insert(&b"CMS.INITBYDIM"[..], CmdType::Write);
    cmds_hashmap.insert(&b"CMS.INITBYPROB"[..], CmdType::Write);
    cmds_hashmap.insert(&b"CMS.MERGE"[..], CmdType::Write);
    cmds_hashmap.insert(&b"CMS.QUERY"[..], CmdType::Read);

    // TopK commands.
    cmds_hashmap.insert(&b"TOPK.ADD"[..], CmdType::Write);
    cmds_hashmap.insert(&b"TOPK.COUNT"[..], CmdType::Read);
    cmds_hashmap.insert(&b"TOPK.INCRBY"[..], CmdType::Write);
    cmds_hashmap.insert(&b"TOPK.INFO"[..], CmdType::Read);
    cmds_hashmap.insert(&b"TOPK.LIST"[..], CmdType::Read);
    cmds_hashmap.insert(&b"TOPK.QUERY"[..], CmdType::Read);
    cmds_hashmap.insert(&b"TOPK.RESERVE"[..], CmdType::Write);

    // T-digest Sketch commands.
    cmds_hashmap.insert(&b"TDIGEST.ADD"[..], CmdType::Write);
    cmds_hashmap.insert(&b"TDIGEST.BYRANK"[..], CmdType::Read);
    cmds_hashmap.insert(&b"TDIGEST.BYREVRANK"[..], CmdType::Read);
    cmds_hashmap.insert(&b"TDIGEST.CDF"[..], CmdType::Read);
    cmds_hashmap.insert(&b"TDIGEST.CREATE"[..], CmdType::Write);
    cmds_hashmap.insert(&b"TDIGEST.INFO"[..], CmdType::Read);
    cmds_hashmap.insert(&b"TDIGEST.MAX"[..], CmdType::Read);
    cmds_hashmap.insert(&b"TDIGEST.MIN"[..], CmdType::Read);
    cmds_hashmap.insert(&b"TDIGEST.QUANTILE"[..], CmdType::Read);
    cmds_hashmap.insert(&b"TDIGEST.RANK"[..], CmdType::Read);
    cmds_hashmap.insert(&b"TDIGEST.REVRANK"[..], CmdType::Read);
    cmds_hashmap.insert(&b"TDIGEST.MERGE"[..], CmdType::Write);
    cmds_hashmap.insert(&b"TDIGEST.RESET"[..], CmdType::Write);
    cmds_hashmap.insert(&b"TDIGEST.TRIMMED_MEAN"[..], CmdType::Read);

    let _ = CMD_HASHMAP.set(cmds_hashmap);
}

impl CmdType {
    pub fn is_read(self) -> bool {
        CmdType::Read == self || self.is_mget() || self.is_exists() // || self.is_keys() || self.is_dbsize()
    }

    pub fn is_write(self) -> bool {
        CmdType::Write == self
    }

    pub fn is_mget(self) -> bool {
        CmdType::MGet == self
    }

    pub fn is_mset(self) -> bool {
        CmdType::MSet == self
    }

    pub fn is_exists(self) -> bool {
        CmdType::Exists == self
    }

    pub fn is_eval(self) -> bool {
        CmdType::Eval == self
    }

    pub fn is_del(self) -> bool {
        CmdType::Del == self
    }

    pub fn is_not_support(self) -> bool {
        CmdType::NotSupport == self
    }

    pub fn is_ctrl(self) -> bool {
        CmdType::Ctrl == self
    }

    pub fn is_auth(self) -> bool {
        CmdType::Auth == self
    }

    pub fn is_info(self) -> bool {
        CmdType::Info == self
    }

    pub fn is_read_all(self) -> bool {
        CmdType::ReadAll == self
    }

    pub fn is_count_all(self) -> bool {
        CmdType::CountAll == self
    }

    pub fn is_command(self) -> bool {
        CmdType::Command == self
    }

    pub fn is_client(self) -> bool {
        CmdType::Client == self
    }

    pub fn is_module(self) -> bool {
        CmdType::Module == self
    }

    pub fn is_scan(self) -> bool {
        CmdType::Scan == self
    }

    pub fn is_memory(self) -> bool {
        CmdType::Memory == self
    }

    pub fn need_auth(self) -> bool {
        self.is_read()
            || self.is_write()
            || self.is_mget()
            || self.is_mset()
            || self.is_exists()
            || self.is_eval()
            || self.is_del()
            || self.is_ctrl()
            || self.is_read_all()
            || self.is_count_all()
            || self.is_scan()
    }

    pub fn get_cmd_type(msg: &Message) -> CmdType {
        if let Some(data) = msg.nth(0) {
            if let Some(ctype) = CMD_HASHMAP.get().unwrap().get(data) {
                return *ctype;
            }
        }
        CmdType::NotSupport
    }
}
