use std::future::Ready;

use futures::stream::BoxStream;
use rorm_sql::value::Value;

use crate::error::Error;
use crate::query_type::{All, Nothing, One, Optional, QueryType, Stream};
use crate::row::Row;

use super::no_sqlx;

impl QueryType for One {
    type Future<'result> = Ready<Result<Row, Error>>;

    fn query<E>(_executor: E, _query: String, _values: Vec<Value>) -> Self::Future<'static> {
        no_sqlx();
    }
}

impl QueryType for All {
    type Future<'result> = Ready<Result<Vec<Row>, Error>>;

    fn query<E>(_executor: E, _query: String, _values: Vec<Value>) -> Self::Future<'static> {
        no_sqlx();
    }
}

impl QueryType for Optional {
    type Future<'result> = Ready<Result<Option<Row>, Error>>;

    fn query<E>(_executor: E, _query: String, _values: Vec<Value>) -> Self::Future<'static> {
        no_sqlx();
    }
}

impl QueryType for Stream {
    type Future<'result> = BoxStream<'result, Result<Row, Error>>;

    fn query<E>(_executor: E, _query: String, _values: Vec<Value>) -> Self::Future<'static> {
        no_sqlx();
    }
}

impl QueryType for Nothing {
    type Future<'result> = Ready<Result<(), Error>>;

    fn query<E>(_executor: E, _query: String, _values: Vec<Value>) -> Self::Future<'static> {
        no_sqlx();
    }
}
