//! Simple alternative to [`sqlx::any`] which is tailored for rorm at the loss of generality.
//!
//! **Beware:**
//! `AnyQuery<'q>` and `AnyExecutor<'e>` work quite different than `Query<'q, Any, _>` and `Executor<'e, Database = Any>`

use std::ops::DerefMut;

use futures::stream::BoxStream;
use futures::{StreamExt, TryStreamExt};
use sqlx::query::Query;
use sqlx::{Executor, Pool, Transaction};

#[macro_use]
#[path = "./cond_macros.rs"]
mod cond_macros;

#[cfg(feature = "mysql")]
use sqlx::{mysql, MySql};
#[cfg(feature = "postgres")]
use sqlx::{postgres, Postgres};
#[cfg(feature = "sqlite")]
use sqlx::{sqlite, Sqlite};

/// Enum around [`Pool<DB>`]
#[derive(Clone, Debug)]
pub enum AnyPool {
    #[cfg(feature = "postgres")]
    Postgres(Pool<Postgres>),
    #[cfg(feature = "mysql")]
    MySql(Pool<MySql>),
    #[cfg(feature = "sqlite")]
    Sqlite(Pool<Sqlite>),
}

impl AnyPool {
    /// Retrieves a connection and immediately begins a new transaction.
    ///
    /// See [`Pool::begin`]
    pub async fn begin(&self) -> sqlx::Result<AnyTransaction> {
        match self {
            #[cfg(feature = "postgres")]
            Self::Postgres(pool) => pool.begin().await.map(AnyTransaction::Postgres),
            #[cfg(feature = "mysql")]
            Self::MySql(pool) => pool.begin().await.map(AnyTransaction::MySql),
            #[cfg(feature = "sqlite")]
            Self::Sqlite(pool) => pool.begin().await.map(AnyTransaction::Sqlite),
        }
    }
}

/// Enum around [`Transaction<'static, DB>`]
pub enum AnyTransaction {
    #[cfg(feature = "postgres")]
    Postgres(Transaction<'static, Postgres>),
    #[cfg(feature = "mysql")]
    MySql(Transaction<'static, MySql>),
    #[cfg(feature = "sqlite")]
    Sqlite(Transaction<'static, Sqlite>),
}

impl AnyTransaction {
    /// Commits this transaction or savepoint.
    ///
    /// See [Transaction::commit]
    pub async fn commit(self) -> sqlx::Result<()> {
        match self {
            #[cfg(feature = "postgres")]
            Self::Postgres(tx) => tx.commit().await,
            #[cfg(feature = "mysql")]
            Self::MySql(tx) => tx.commit().await,
            #[cfg(feature = "sqlite")]
            Self::Sqlite(tx) => tx.commit().await,
        }
    }

    /// Aborts this transaction or savepoint.
    ///
    /// See [Transaction::rollback]
    pub async fn rollback(self) -> sqlx::Result<()> {
        match self {
            #[cfg(feature = "postgres")]
            Self::Postgres(tx) => tx.rollback().await,
            #[cfg(feature = "mysql")]
            Self::MySql(tx) => tx.rollback().await,
            #[cfg(feature = "sqlite")]
            Self::Sqlite(tx) => tx.rollback().await,
        }
    }
}

/// Combination of an [`AnyExecutor`] and its associated [`Query<'q, DB, _>`]
pub enum AnyQuery<'q> {
    #[cfg(feature = "postgres")]
    PostgresPool(AnyQueryInner<'q, &'q Pool<Postgres>, postgres::PgArguments>),
    #[cfg(feature = "mysql")]
    MySqlPool(AnyQueryInner<'q, &'q Pool<MySql>, mysql::MySqlArguments>),
    #[cfg(feature = "sqlite")]
    SqlitePool(AnyQueryInner<'q, &'q Pool<Sqlite>, sqlite::SqliteArguments<'q>>),
    #[cfg(feature = "postgres")]
    PostgresConn(AnyQueryInner<'q, &'q mut postgres::PgConnection, postgres::PgArguments>),
    #[cfg(feature = "mysql")]
    MySqlConn(AnyQueryInner<'q, &'q mut mysql::MySqlConnection, mysql::MySqlArguments>),
    #[cfg(feature = "sqlite")]
    SqliteConn(AnyQueryInner<'q, &'q mut sqlite::SqliteConnection, sqlite::SqliteArguments<'q>>),
}
#[doc(hidden)]
pub struct AnyQueryInner<'q, E: Executor<'q>, A> {
    executor: E,
    query: Option<Query<'q, E::Database, A>>,
}

impl<'q> AnyQuery<'q> {
    /// Bind a value for use with this SQL query.
    ///
    /// See [`Query::bind`]
    pub fn bind<T>(&mut self, value: T)
    where
        T: 'q + Send + AnyEncode<'q> + AnyType,
    {
        match self {
            #[cfg(feature = "postgres")]
            Self::PostgresPool(AnyQueryInner { query, .. })
            | Self::PostgresConn(AnyQueryInner { query, .. }) => {
                *query = query.take().map(|query| query.bind(value))
            }
            #[cfg(feature = "mysql")]
            Self::MySqlPool(AnyQueryInner { query, .. })
            | Self::MySqlConn(AnyQueryInner { query, .. }) => {
                *query = query.take().map(|query| query.bind(value))
            }
            #[cfg(feature = "sqlite")]
            Self::SqlitePool(AnyQueryInner { query, .. })
            | Self::SqliteConn(AnyQueryInner { query, .. }) => {
                *query = query.take().map(|query| query.bind(value))
            }
        }
    }

    /// Execute the query and return the generated results in a stream.
    pub fn fetch_many(self) -> BoxStream<'q, sqlx::Result<sqlx::Either<AnyQueryResult, AnyRow>>> {
        macro_rules! match_impl {
            ($($variant:ident, $db:ident),+) => {
                match self {$(
                    Self::$variant(AnyQueryInner { executor, query }) => executor
                        .fetch_many(query.unwrap())
                        .map_ok(|either| {
                            either
                                .map_left(AnyQueryResult::$db)
                                .map_right(AnyRow::$db)
                        })
                        .boxed(),
                )+}
            }
        }
        expand_fetch_impl!(match_impl)
    }

    /// Execute the query and return all the generated results, collected into a `Vec`.
    pub async fn fetch_all(self) -> sqlx::Result<Vec<AnyRow>> {
        macro_rules! match_impl {
            ($($variant:ident, $db:ident),+) => {
                match self {$(
                    Self::$variant(AnyQueryInner { executor, query }) => executor
                        .fetch_many(query.unwrap())
                        .try_filter_map(|either| async move {
                            Ok(either
                                .right()
                                .map(AnyRow::$db))
                        })
                        .try_collect()
                        .await,
                )+}
            }
        }
        expand_fetch_impl!(match_impl)
    }

    /// Execute the query and returns at most one row.
    pub async fn fetch_optional(self) -> sqlx::Result<Option<AnyRow>> {
        macro_rules! match_impl {
            ($($variant:ident, $db:ident),+) => {
                match self {$(
                    Self::$variant(AnyQueryInner { executor, query }) => executor
                        .fetch_optional(query.unwrap())
                        .await
                        .map(|option| option.map(AnyRow::$db)),
                )+}
            }
        }
        expand_fetch_impl!(match_impl)
    }

    /// Execute the query and return the number of affected rows.
    pub async fn fetch_affected_rows(self) -> sqlx::Result<u64> {
        macro_rules! match_impl {
            ($($variant:ident, $db:ident),+) => {
                match self {$(
                    Self::$variant(AnyQueryInner { executor, query }) => executor
                        .fetch_many(query.unwrap())
                        .try_fold(0, |sum, either| async move {Ok(match either {
                            sqlx::Either::Left(result) => sum + result.rows_affected(),
                            sqlx::Either::Right(_) => sum,
                        })})
                        .await,
                )+}
            }
        }
        expand_fetch_impl!(match_impl)
    }
}

/// Enum around [`<DB as Database>::Row`](Database#associatedtype.Row)
pub enum AnyRow {
    #[cfg(feature = "postgres")]
    Postgres(postgres::PgRow),
    #[cfg(feature = "mysql")]
    MySql(mysql::MySqlRow),
    #[cfg(feature = "sqlite")]
    Sqlite(sqlite::SqliteRow),
}

/// Enum around [`<DB as Database>::QueryResult`](Database#associatedtype.Row)
pub enum AnyQueryResult {
    #[cfg(feature = "postgres")]
    Postgres(postgres::PgQueryResult),
    #[cfg(feature = "mysql")]
    MySql(mysql::MySqlQueryResult),
    #[cfg(feature = "sqlite")]
    Sqlite(sqlite::SqliteQueryResult),
}

impl AnyQueryResult {
    /// The number of rows affected by a query
    pub fn rows_affected(&self) -> u64 {
        match self {
            #[cfg(feature = "postgres")]
            Self::Postgres(result) => result.rows_affected(),
            #[cfg(feature = "mysql")]
            Self::MySql(result) => result.rows_affected(),
            #[cfg(feature = "sqlite")]
            Self::Sqlite(result) => result.rows_affected(),
        }
    }
}

/// Trait to start queries from either an [`AnyPool`] or an [`AnyTransaction`]
pub trait AnyExecutor<'e> {
    /// Start a query
    ///
    /// This will consume the executor and store it alongside the query until it is executed.
    // This way there is no additional check required whether the executor's db matches the query's
    fn query<'q>(self, query: &'q str) -> AnyQuery<'q>
    where
        'e: 'q;
}
impl<'e> AnyExecutor<'e> for &'e AnyPool {
    fn query<'q>(self, query: &'q str) -> AnyQuery<'q>
    where
        'e: 'q,
    {
        match self {
            #[cfg(feature = "postgres")]
            AnyPool::Postgres(pool) => AnyQuery::PostgresPool(AnyQueryInner {
                executor: pool,
                query: Some(sqlx::query(query)),
            }),
            #[cfg(feature = "mysql")]
            AnyPool::MySql(pool) => AnyQuery::MySqlPool(AnyQueryInner {
                executor: pool,
                query: Some(sqlx::query(query)),
            }),
            #[cfg(feature = "sqlite")]
            AnyPool::Sqlite(pool) => AnyQuery::SqlitePool(AnyQueryInner {
                executor: pool,
                query: Some(sqlx::query(query)),
            }),
        }
    }
}
impl<'e> AnyExecutor<'e> for &'e mut AnyTransaction {
    fn query<'q>(self, query: &'q str) -> AnyQuery<'q>
    where
        'e: 'q,
    {
        match self {
            #[cfg(feature = "postgres")]
            AnyTransaction::Postgres(tx) => AnyQuery::PostgresConn(AnyQueryInner {
                executor: tx.deref_mut(),
                query: Some(sqlx::query(query)),
            }),
            #[cfg(feature = "mysql")]
            AnyTransaction::MySql(tx) => AnyQuery::MySqlConn(AnyQueryInner {
                executor: tx.deref_mut(),
                query: Some(sqlx::query(query)),
            }),
            #[cfg(feature = "sqlite")]
            AnyTransaction::Sqlite(tx) => AnyQuery::SqliteConn(AnyQueryInner {
                executor: tx.deref_mut(),
                query: Some(sqlx::query(query)),
            }),
        }
    }
}

macro_rules! uncond_trait_alias {
    ($(#[doc = $doc:literal])* trait $trait:ident $(<$lifetime:lifetime>)?: $($bound:path,)+) => {
        $(#[doc = $doc])*
        pub trait $trait $(<$lifetime>)?
        where
            $(Self: $bound),+
        {}

        impl<$($lifetime,)? T> $trait $(<$lifetime>)? for T
        where
            $(Self: $bound),+
        {}
    };
}

trait_alias!(
    /// Trait alias combining all [`Encode<'q, DB>`](sqlx::Encode)
    trait AnyEncode<'q>: sqlx::Encode<'q, Postgres>, sqlx::Encode<'q, MySql>, sqlx::Encode<'q, Sqlite>,
);

trait_alias!(
    /// Trait alias combining all [`Decode<'r, DB>`](sqlx::Decode)
    trait AnyDecode<'r>: sqlx::Decode<'r, Postgres>, sqlx::Decode<'r, MySql>, sqlx::Decode<'r, Sqlite>,
);

trait_alias!(
    /// Trait alias combining all [`Type<DB>`](sqlx::Type)
    trait AnyType: sqlx::Type<Postgres>, sqlx::Type<MySql>, sqlx::Type<Sqlite>,
);

trait_alias!(
    /// Trait alias combining all [`ColumnIndex<DB>`](sqlx::ColumnIndex)
    trait AnyColumnIndex: sqlx::ColumnIndex<postgres::PgRow>, sqlx::ColumnIndex<mysql::MySqlRow>, sqlx::ColumnIndex<sqlite::SqliteRow>,
);
