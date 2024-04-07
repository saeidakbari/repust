pub mod meta;
// Path: src/com/meta.rs

pub mod config;
// Path: src/com/config.rs

use std::num;
use thiserror::Error;
use toml::de::Error as TOMLError;

#[derive(Debug, Error)]
pub enum AsError {
    #[error("config is bad for fields {}", _0)]
    BadConfig(String),

    #[error("fail to parse int in config")]
    StrParseIntError(num::ParseIntError),

    #[error("invalid message")]
    BadMessage,

    #[error("message is ok but request bad or not allowed")]
    BadRequest,

    #[error("request not supported")]
    RequestNotSupport,

    #[error("NOAUTH Authentication required.")]
    NoAuth,

    #[error("WRONGPASS invalid username-password pair or user is disabled.")]
    AuthWrong,

    #[error("inline request don't support multi keys")]
    RequestInlineWithMultiKeys,

    #[error("message reply is bad")]
    BadReply,

    #[error("command timeout")]
    CmdTimeout,

    #[error("proxy fail")]
    ProxyFail,

    #[error("connection closed of {}", _0)]
    ConnClosed(String),

    #[error("fail due retry send, reached limit")]
    RequestReachMaxCycle,

    #[error("fail to parse integer {}", _0)]
    ParseIntError(btoi::ParseIntegerError),

    #[error("CLUSTER SLOTS must be replied with array")]
    WrongClusterSlotsReplyType,

    #[error("CLUSTER SLOTS must contains slot info")]
    WrongClusterSlotsReplySlot,

    #[error("cluster fail to proxy command")]
    ClusterFailDispatch,

    #[error("unexpected io error {}", _0)]
    IoError(tokio::io::Error), // io_error

    #[error("remote connection has been active closed: {}", _0)]
    BackendClosedError(String),

    #[error("fail to redirect command")]
    RedirectFailError,

    #[error("fail to init cluster {} due to all seed nodes is die", _0)]
    ClusterAllSeedsDie(String),

    #[error("fail to load config toml error {}", _0)]
    ConfigError(TOMLError), // de error

    #[error("fail to load system info")]
    SystemError,

    #[error("there is nothing happening")]
    None,
}

impl PartialEq for AsError {
    fn eq(&self, other: &AsError) -> bool {
        match (self, other) {
            (Self::None, Self::None) => true,
            (Self::BadMessage, Self::BadMessage) => true,
            (Self::BadRequest, Self::BadRequest) => true,
            (Self::RequestNotSupport, Self::RequestNotSupport) => true,
            (Self::NoAuth, Self::NoAuth) => true,
            (Self::AuthWrong, Self::AuthWrong) => true,
            (Self::RequestInlineWithMultiKeys, Self::RequestInlineWithMultiKeys) => true,
            (Self::BadReply, Self::BadReply) => true,
            (Self::ProxyFail, Self::ProxyFail) => true,
            (Self::RequestReachMaxCycle, Self::RequestReachMaxCycle) => true,
            (Self::WrongClusterSlotsReplyType, Self::WrongClusterSlotsReplyType) => true,
            (Self::WrongClusterSlotsReplySlot, Self::WrongClusterSlotsReplySlot) => true,
            (Self::ClusterFailDispatch, Self::ClusterFailDispatch) => true,
            (Self::RedirectFailError, Self::RedirectFailError) => true,
            (Self::ParseIntError(inner), Self::ParseIntError(other_inner)) => inner == other_inner,
            (Self::BackendClosedError(inner), Self::BackendClosedError(other_inner)) => {
                inner == other_inner
            }
            (Self::StrParseIntError(inner), Self::StrParseIntError(other_inner)) => {
                inner == other_inner
            }
            (Self::ClusterAllSeedsDie(inner), Self::ClusterAllSeedsDie(other_inner)) => {
                inner == other_inner
            }

            (Self::IoError(inner), Self::IoError(other_inner)) => {
                inner.kind() == other_inner.kind()
            }
            (Self::ConfigError(_), Self::ConfigError(_)) => true,
            (Self::SystemError, Self::SystemError) => true,
            (Self::ConnClosed(addr1), Self::ConnClosed(addr2)) => addr1 == addr2,

            // Not defined errors are always false
            _ => false,
        }
    }
}

impl From<tokio::io::Error> for AsError {
    fn from(oe: tokio::io::Error) -> AsError {
        AsError::IoError(oe)
    }
}

impl From<btoi::ParseIntegerError> for AsError {
    fn from(oe: btoi::ParseIntegerError) -> AsError {
        AsError::ParseIntError(oe)
    }
}

impl From<toml::de::Error> for AsError {
    fn from(oe: toml::de::Error) -> AsError {
        AsError::ConfigError(oe)
    }
}

impl From<num::ParseIntError> for AsError {
    fn from(oe: num::ParseIntError) -> AsError {
        AsError::StrParseIntError(oe)
    }
}
