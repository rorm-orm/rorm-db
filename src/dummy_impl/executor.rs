use std::future::Ready;

use futures::stream::Empty;
use rorm_sql::value::Value;
use rorm_sql::DBImpl;

use crate::database::Database;
use crate::error::Error;
use crate::executor::{
    AffectedRows, All, Executor, Nothing, One, Optional, QueryStrategy, QueryStrategyResult, Stream,
};
use crate::row::Row;
use crate::transaction::Transaction;

use super::no_sqlx;

impl<'executor> Executor<'executor> for &'executor Database {
    fn execute<'data, 'result, Q>(
        self,
        _query: String,
        _values: Vec<Value<'data>>,
    ) -> Q::Result<'result>
    where
        'executor: 'result,
        'data: 'result,
        Q: QueryStrategy,
    {
        no_sqlx();
    }

    fn dialect(&self) -> DBImpl {
        self.db_impl
    }

    type EnsureTransactionFuture = Ready<Result<TransactionGuard<'executor>, Error>>;

    fn ensure_transaction(self) -> Self::EnsureTransactionFuture {
        no_sqlx();
    }
}
impl<'executor> Executor<'executor> for &'executor mut Transaction<'_> {
    fn execute<'data, 'result, Q>(
        self,
        _query: String,
        _values: Vec<Value<'data>>,
    ) -> Q::Result<'result>
    where
        'executor: 'result,
        'data: 'result,
        Q: QueryStrategy,
    {
        no_sqlx();
    }

    fn dialect(&self) -> DBImpl {
        self.db_impl
    }

    type EnsureTransactionFuture = Ready<Result<TransactionGuard<'executor>, Error>>;

    fn ensure_transaction(self) -> Self::EnsureTransactionFuture {
        no_sqlx();
    }
}

pub trait QueryStrategyImpl: QueryStrategyResult {}
impl<Q: QueryStrategyResult> QueryStrategyImpl for Q {}

impl QueryStrategyResult for Nothing {
    type Result<'result> = Ready<Result<(), Error>>;
}

impl QueryStrategyResult for AffectedRows {
    type Result<'result> = Ready<Result<u64, Error>>;
}

impl QueryStrategyResult for One {
    type Result<'result> = Ready<Result<Row, Error>>;
}

impl QueryStrategyResult for Optional {
    type Result<'result> = Ready<Result<Option<Row>, Error>>;
}

impl QueryStrategyResult for All {
    type Result<'result> = Ready<Result<Vec<Row>, Error>>;
}

impl QueryStrategyResult for Stream {
    type Result<'result> = Empty<Result<Row, Error>>;
}
