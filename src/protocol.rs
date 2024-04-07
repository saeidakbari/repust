pub mod mc;
// Path: src/protocol/mc.rs

pub mod redis;
// Path: src/protocol/redis.rs

use bitflags::bitflags;

pub trait IntoReply<R> {
    fn into_reply(self) -> R;
}

impl<T> IntoReply<T> for T {
    fn into_reply(self) -> T {
        self
    }
}

bitflags! {
    #[derive(Clone,Copy, Debug, PartialEq, Eq)]
    pub struct CmdFlags: u8 {
        const DONE     = 0b00_000_001;
        // redis cluster only
        const ASK      = 0b00_000_010;
        const MOVED    = 0b00_000_100;
        // mc only
        const NOREPLY  = 0b00_001_000;
        const QUIET    = 0b00_010_000;

        // retry
        const RETRY    = 0b00100000;

        const ERROR    = 0b10_000_000;
    }
}

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum CmdType {
    Read,
    Write,
    Ctrl,
    NotSupport,

    // These commands are specific to Redis.
    MSet,     // Write
    MGet,     // Read
    Exists,   // Read
    Eval,     // Write
    Del,      // Write
    Auth,     // Auth
    Info,     // Read
    ReadAll,  // ReadAll
    CountAll, // CountAll
    Command,  // Command
    Client,   // Client
    Module,   // Module
    Scan,     // Scan
    Memory,   // Memory
}
