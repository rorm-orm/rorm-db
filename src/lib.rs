//! This crate is used as language independent base for building an orm.
//!
//! Rust specific features will be exposed through the `rorm` crate.
//! `rorm-lib` implements C bindings for this crate.
#![warn(missing_docs)]

#[cfg(any(
    all(
        feature = "actix-rustls",
        any(
            feature = "actix-native-tls",
            feature = "tokio-native-tls",
            feature = "tokio-rustls",
            feature = "async-std-native-tls",
            feature = "async-std-rustls"
        )
    ),
    all(
        feature = "actix-native-tls",
        any(
            feature = "tokio-native-tls",
            feature = "tokio-rustls",
            feature = "async-std-native-tls",
            feature = "async-std-rustls"
        )
    ),
    all(
        feature = "tokio-rustls",
        any(
            feature = "tokio-native-tls",
            feature = "async-std-native-tls",
            feature = "async-std-rustls"
        )
    ),
    all(
        feature = "tokio-native-tls",
        any(feature = "async-std-native-tls", feature = "async-std-rustls")
    ),
    all(feature = "async-std-native-tls", feature = "async-std-rustls")
))]
compile_error!("Using multiple runtime / tls configurations at the same time is not allowed");

pub mod database;
pub mod error;

pub(crate) mod query_type;

pub mod executor;
pub mod row;
pub mod transaction;

#[cfg(feature = "sqlx")]
pub(crate) mod utils;

#[cfg_attr(feature = "sqlx", path = "sqlx_impl/mod.rs")]
#[cfg_attr(not(feature = "sqlx"), path = "dummy_impl/mod.rs")]
pub(crate) mod internal;

pub use rorm_declaration::config::DatabaseDriver;
pub use rorm_sql::{
    aggregation, and, conditional, join_table, limit_clause, or, ordering, select_column, value,
};

pub use crate::{
    database::{Database, DatabaseConfiguration},
    error::Error,
    row::Row,
};
