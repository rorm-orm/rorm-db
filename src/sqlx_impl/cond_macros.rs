#[rustfmt::skip]
#[cfg(all(feature = "postgres", feature = "mysql", feature = "sqlite"))]
macro_rules! trait_alias {
    ($(#[doc = $doc:literal])* trait $trait:ident $(<$lifetime:lifetime>)?: $postgres:path, $mysql:path, $sqlite:path,) => {
        uncond_trait_alias!($(#[doc = $doc])* trait $trait $(<$lifetime>)?: $postgres, $mysql, $sqlite,);
    };
}

#[rustfmt::skip]
#[cfg(all(not(feature = "postgres"), feature = "mysql", feature = "sqlite"))]
macro_rules! trait_alias {
    ($(#[doc = $doc:literal])* trait $trait:ident $(<$lifetime:lifetime>)?: $postgres:path, $mysql:path, $sqlite:path,) => {
        uncond_trait_alias!($(#[doc = $doc])* trait $trait $(<$lifetime>)?: $mysql, $sqlite,);
    };
}

#[rustfmt::skip]
#[cfg(all(feature = "postgres", not(feature = "mysql"), feature = "sqlite"))]
macro_rules! trait_alias {
    ($(#[doc = $doc:literal])* trait $trait:ident $(<$lifetime:lifetime>)?: $postgres:path, $mysql:path, $sqlite:path,) => {
        uncond_trait_alias!($(#[doc = $doc])* trait $trait $(<$lifetime>)?: $postgres, $sqlite,);
    };
}

#[rustfmt::skip]
#[cfg(all(not(feature = "postgres"), not(feature = "mysql"), feature = "sqlite"))]
macro_rules! trait_alias {
    ($(#[doc = $doc:literal])* trait $trait:ident $(<$lifetime:lifetime>)?: $postgres:path, $mysql:path, $sqlite:path,) => {
        uncond_trait_alias!($(#[doc = $doc])* trait $trait $(<$lifetime>)?: $sqlite,);
    };
}

#[rustfmt::skip]
#[cfg(all(feature = "postgres", feature = "mysql", not(feature = "sqlite")))]
macro_rules! trait_alias {
    ($(#[doc = $doc:literal])* trait $trait:ident $(<$lifetime:lifetime>)?: $postgres:path, $mysql:path, $sqlite:path,) => {
        uncond_trait_alias!($(#[doc = $doc])* trait $trait $(<$lifetime>)?: $postgres, $mysql,);
    };
}

#[rustfmt::skip]
#[cfg(all(not(feature = "postgres"), feature = "mysql", not(feature = "sqlite")))]
macro_rules! trait_alias {
    ($(#[doc = $doc:literal])* trait $trait:ident $(<$lifetime:lifetime>)?: $postgres:path, $mysql:path, $sqlite:path,) => {
        uncond_trait_alias!($(#[doc = $doc])* trait $trait $(<$lifetime>)?: $mysql,);
    };
}

#[rustfmt::skip]
#[cfg(all(feature = "postgres", not(feature = "mysql"), not(feature = "sqlite")))]
macro_rules! trait_alias {
    ($(#[doc = $doc:literal])* trait $trait:ident $(<$lifetime:lifetime>)?: $postgres:path, $mysql:path, $sqlite:path,) => {
        uncond_trait_alias!($(#[doc = $doc])* trait $trait $(<$lifetime>)?: $postgres,);
    };
}

#[rustfmt::skip]
#[cfg(all(feature = "postgres", feature = "mysql", feature = "sqlite"))]
macro_rules! expand_fetch_impl {
    ($macro:ident) => {
        $macro!(PostgresPool, Postgres, PostgresConn, Postgres, MySqlPool, MySql, MySqlConn, MySql, SqlitePool, Sqlite, SqliteConn, Sqlite)
    };
}
#[rustfmt::skip]
#[cfg(all(not(feature = "postgres"), feature = "mysql", feature = "sqlite"))]
macro_rules! expand_fetch_impl {
    ($macro:ident) => {
        $macro!(MySqlPool, MySql, MySqlConn, MySql, SqlitePool, Sqlite, SqliteConn, Sqlite)
    };
}
#[rustfmt::skip]
#[cfg(all(feature = "postgres", not(feature = "mysql"), feature = "sqlite"))]
macro_rules! expand_fetch_impl {
    ($macro:ident) => {
        $macro!(PostgresPool, Postgres, PostgresConn, Postgres, SqlitePool, Sqlite, SqliteConn, Sqlite)
    };
}
#[rustfmt::skip]
#[cfg(all(not(feature = "postgres"), not(feature = "mysql"), feature = "sqlite"))]
macro_rules! expand_fetch_impl {
    ($macro:ident) => {
        $macro!(SqlitePool, Sqlite, SqliteConn, Sqlite)
    };
}
#[rustfmt::skip]
#[cfg(all(feature = "postgres", feature = "mysql", not(feature = "sqlite")))]
macro_rules! expand_fetch_impl {
    ($macro:ident) => {
        $macro!(PostgresPool, Postgres, PostgresConn, Postgres, MySqlPool, MySql, MySqlConn, MySql)
    };
}
#[rustfmt::skip]
#[cfg(all(not(feature = "postgres"), feature = "mysql", not(feature = "sqlite")))]
macro_rules! expand_fetch_impl {
    ($macro:ident) => {
        $macro!(MySqlPool, MySql, MySqlConn, MySql)
    };
}
#[rustfmt::skip]
#[cfg(all(feature = "postgres", not(feature = "mysql"), not(feature = "sqlite")))]
macro_rules! expand_fetch_impl {
    ($macro:ident) => {
        $macro!(PostgresPool, Postgres, PostgresConn, Postgres)
    };
}
