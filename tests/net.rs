use std::net::SocketAddr;
use miningsim::net::{self, CliParseError, NetMode, DEFAULT_PORT};

fn s(args: &[&str]) -> Vec<String> {
    args.iter().map(|s| s.to_string()).collect()
}

#[test]
fn no_args_is_single_player() {
    assert_eq!(net::parse_args(&[]), Ok(NetMode::SinglePlayer));
}

#[test]
fn host_no_port_uses_default() {
    assert_eq!(net::parse_args(&s(&["host"])), Ok(NetMode::Host { port: DEFAULT_PORT }));
}

#[test]
fn host_with_port() {
    assert_eq!(net::parse_args(&s(&["host", "5050"])), Ok(NetMode::Host { port: 5050 }));
}

#[test]
fn host_with_bad_port() {
    assert_eq!(
        net::parse_args(&s(&["host", "abc"])),
        Err(CliParseError::BadPort("abc".to_string())),
    );
}

#[test]
fn join_with_addr() {
    let expected: SocketAddr = "192.168.1.5:5000".parse().unwrap();
    assert_eq!(net::parse_args(&s(&["join", "192.168.1.5:5000"])), Ok(NetMode::Client { addr: expected }));
}

#[test]
fn join_with_loopback() {
    let expected: SocketAddr = "127.0.0.1:5000".parse().unwrap();
    assert_eq!(net::parse_args(&s(&["join", "127.0.0.1:5000"])), Ok(NetMode::Client { addr: expected }));
}

#[test]
fn join_missing_addr() {
    assert_eq!(
        net::parse_args(&s(&["join"])),
        Err(CliParseError::MissingArg("join requires an address")),
    );
}

#[test]
fn join_bad_addr() {
    assert!(matches!(
        net::parse_args(&s(&["join", "not-an-addr"])),
        Err(CliParseError::BadAddr(_)),
    ));
}

#[test]
fn unknown_command() {
    assert_eq!(
        net::parse_args(&s(&["whatever"])),
        Err(CliParseError::UnknownCommand("whatever".to_string())),
    );
}
