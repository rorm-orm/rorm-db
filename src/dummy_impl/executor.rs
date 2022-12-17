use std::future::Ready;

use futures::stream::Empty;
use rorm_sql::value::Value;

use crate::database::Database;
use crate::error::Error;
use crate::executor::{All, Executor, Nothing, One, Optional, QueryStrategy, Stream};
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
}

pub trait QueryStrategyImpl {
    type Result<'a>;
}

impl QueryStrategyImpl for Nothing {
    type Result<'result> = Ready<Result<(), Error>>;
}

impl QueryStrategyImpl for One {
    type Result<'result> = Ready<Result<Row, Error>>;
}

impl QueryStrategyImpl for Optional {
    type Result<'result> = Ready<Result<Option<Row>, Error>>;
}

impl QueryStrategyImpl for All {
    type Result<'result> = Ready<Result<Vec<Row>, Error>>;
}

impl QueryStrategyImpl for Stream {
    type Result<'result> = Empty<Result<Row, Error>>;
}
