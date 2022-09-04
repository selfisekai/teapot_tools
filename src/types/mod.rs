pub mod cipd {
    include!(concat!(env!("OUT_DIR"), "/cipd.repository.rs"));
}
pub mod deps;
pub mod dotgclient;
pub mod machine;
