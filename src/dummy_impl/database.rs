use rorm_sql::value::Value;

use super::{no_sqlx, NotInstantiable};
use crate::database::{Database, DatabaseConfiguration};
use crate::error::Error;
use crate::row::Row;
use crate::transaction::Transaction;

pub(crate) type Impl = NotInstantiable;

/// Implementation of [Database::connect]
pub(crate) async fn connect(_configuration: DatabaseConfiguration) -> Result<Database, Error> {
    no_sqlx();
}

/// Implementation of [Database::raw_sql]
pub async fn raw_sql<'a>(
    db: &Database,
    _query_string: &'a str,
    _bind_params: Option<&[Value<'a>]>,
    _transaction: Option<&mut Transaction>,
) -> Result<Vec<Row>, Error> {
    // "Read" pool at least once
    let _ = db.pool;
    no_sqlx();
}

/// Implementation of [Database::start_transaction]
pub async fn start_transaction(_db: &Database) -> Result<Transaction, Error> {
    no_sqlx();
}
