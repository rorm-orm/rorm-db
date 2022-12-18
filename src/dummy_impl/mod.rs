pub(crate) mod database;
pub(crate) mod executor;
pub(crate) mod row;
pub(crate) mod transaction;

pub(crate) fn no_sqlx() -> ! {
    panic!(
        r#"
        Your binary was compiled without specifiying a runtime and tls implementation.
        
        One of async-std-native-tls, async-std-rustls, tokio-native-tls, tokio-rustls, 
        actix-native-tls, actix-rustls is required.
        "#
    );
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum NotInstantiable {}
