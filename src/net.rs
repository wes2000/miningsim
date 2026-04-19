use std::net::SocketAddr;
use bevy::prelude::Resource;

#[derive(Debug, Clone, PartialEq, Eq, Resource)]
pub enum NetMode {
    SinglePlayer,
    Host { port: u16 },
    Client { addr: SocketAddr },
}

pub const DEFAULT_PORT: u16 = 5000;

#[derive(Debug, PartialEq, Eq)]
pub enum CliParseError {
    UnknownCommand(String),
    MissingArg(&'static str),
    BadAddr(String),
    BadPort(String),
}

/// Parse `std::env::args()`-style strings (excluding the binary name).
/// Accepts:
///   []                                       → SinglePlayer
///   ["host"]                                 → Host { port: DEFAULT_PORT }
///   ["host", "<port>"]                       → Host { port: <parsed> }
///   ["join", "<addr>"]                       → Client { addr: <parsed> }
pub fn parse_args(args: &[String]) -> Result<NetMode, CliParseError> {
    match args.first().map(String::as_str) {
        None => Ok(NetMode::SinglePlayer),
        Some("host") => {
            let port = match args.get(1) {
                None => DEFAULT_PORT,
                Some(p) => p.parse().map_err(|_| CliParseError::BadPort(p.clone()))?,
            };
            Ok(NetMode::Host { port })
        }
        Some("join") => {
            let addr_str = args.get(1).ok_or(CliParseError::MissingArg("join requires an address"))?;
            let addr = addr_str.parse().map_err(|_| CliParseError::BadAddr(addr_str.clone()))?;
            Ok(NetMode::Client { addr })
        }
        Some(other) => Err(CliParseError::UnknownCommand(other.to_string())),
    }
}
