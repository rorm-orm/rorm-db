use rorm_sql::conditional::Condition;
use rorm_sql::ordering::OrderByEntry;
use rorm_sql::value::Value;

use crate::database::{ColumnSelector, Database, DatabaseConfiguration, JoinTable};
use crate::error::Error;
use crate::query_type::{GetLimitClause, QueryType};
use crate::row::Row;
use crate::transaction::Transaction;

use super::{no_sqlx, NotInstantiable};

pub(crate) type Impl = NotInstantiable;

/// Implementation of [Database::connect]
pub(crate) async fn connect(_configuration: DatabaseConfiguration) -> Result<Database, Error> {
    no_sqlx();
}

/// Generic implementation of:
/// - [Database::query_one]
/// - [Database::query_optional]
/// - [Database::query_all]
/// - [Database::query_stream]
pub(crate) fn query<'result, 'db: 'result, 'post_query: 'result, Q: QueryType + GetLimitClause>(
    db: &'db Database,
    _model: &str,
    _columns: &[ColumnSelector<'_>],
    _joins: &[JoinTable<'_, 'post_query>],
    _conditions: Option<&Condition<'post_query>>,
    _order_by_clause: &[OrderByEntry<'_>],
    _limit: <Q as GetLimitClause>::Input,
    _transaction: Option<&'db mut Transaction<'_>>,
) -> Q::Future<'result> {
    // "Read" pool at least once
    let _ = db.pool;
    no_sqlx();
}

/// Generic implementation of:
/// - [Database::insert]
/// - [Database::insert_returning]
pub(crate) fn insert<'result, 'db: 'result, 'post_query: 'result, Q: QueryType>(
    _db: &'db Database,
    _model: &str,
    _columns: &[&str],
    _values: &[Value<'post_query>],
    _transaction: Option<&'db mut Transaction<'_>>,
    _returning: Option<&[&str]>,
) -> Q::Future<'result> {
    no_sqlx();
}

/// Implementation of [Database::insert_bulk]
pub async fn insert_bulk(
    _db: &Database,
    _model: &str,
    _columns: &[&str],
    _rows: &[&[Value<'_>]],
    _transaction: Option<&mut Transaction<'_>>,
) -> Result<(), Error> {
    no_sqlx();
}

/// Implementation of [Database::insert_bulk_returning]
pub async fn insert_bulk_returning(
    _db: &Database,
    _model: &str,
    _columns: &[&str],
    _rows: &[&[Value<'_>]],
    _transaction: Option<&mut Transaction<'_>>,
    _returning: &[&str],
) -> Result<Vec<Row>, Error> {
    no_sqlx();
}

/// Implementation of [Database::delete]
pub async fn delete<'post_build>(
    _db: &Database,
    _model: &str,
    _condition: Option<&Condition<'post_build>>,
    _transaction: Option<&mut Transaction<'_>>,
) -> Result<u64, Error> {
    no_sqlx();
}

/// Implementation of [Database::update]
pub async fn update<'post_build>(
    _db: &Database,
    _model: &str,
    _updates: &[(&str, Value<'post_build>)],
    _condition: Option<&Condition<'post_build>>,
    _transaction: Option<&mut Transaction<'_>>,
) -> Result<u64, Error> {
    no_sqlx();
}

/// Implementation of [Database::raw_sql]
pub async fn raw_sql<'a>(
    _db: &Database,
    _query_string: &'a str,
    _bind_params: Option<&[Value<'a>]>,
    _transaction: Option<&mut Transaction<'_>>,
) -> Result<Vec<Row>, Error> {
    no_sqlx();
}

/// Implementation of [Database::start_transaction]
pub async fn start_transaction(_db: &Database) -> Result<Transaction, Error> {
    no_sqlx();
}
