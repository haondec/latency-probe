use serde::{Deserialize, Serialize};

pub mod icmp;
pub mod tcp_connect;
pub mod http;
pub mod echo;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProbeKind {
    Icmp,
    TcpConnect,
    Http,
    Echo,
}
