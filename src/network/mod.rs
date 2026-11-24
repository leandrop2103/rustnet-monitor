pub mod adapters;
pub mod alerts;
pub mod bufferbloat;
pub mod detector;
pub mod dns;
pub mod icmp;
pub mod mtr;
pub mod mtu;
pub mod pinger;
pub mod routes;
pub mod stats;
pub mod types;
pub mod virtual_filter;
pub mod wan;
pub mod wifi;

pub use detector::inspect_network;
