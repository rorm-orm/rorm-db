//! Abstraction over which concrete method of [Executor] to use

use std::future::Future;
use std::task::{Context, Poll};

use aliasable::string::AliasableString;
use futures::future::BoxFuture;
use futures::stream::BoxStream;
use futures::StreamExt;
use rorm_sql::limit_clause::LimitClause;
use rorm_sql::value::Value;
use sqlx::any::AnyQueryResult;
use sqlx::Any as AnyDb;
use sqlx::Executor;

use crate::result::QueryStream;
use crate::{utils, Error, Row};

type AnyQuery<'q> = sqlx::query::Query<'q, AnyDb, sqlx::any::AnyArguments<'q>>;

pub(crate) trait GetLimitClause {
    type Input;

    fn get_limit_clause(input: Self::Input) -> Option<LimitClause>;
}

pub(crate) trait QueryType {
    type Future<'result>;

    fn query<'post_query, E>(
        executor: E,
        query: String,
        values: Vec<Value<'post_query>>,
    ) -> Self::Future<'post_query>
    where
        E: Executor<'post_query, Database = AnyDb>;
}

pub(crate) trait QueryByFuture {
    type Sqlx;
    type Rorm;

    fn create_future<'result, 'db: 'result, 'post_query: 'result, E>(
        executor: E,
        query: AnyQuery<'post_query>,
    ) -> BoxFuture<'result, Result<Self::Sqlx, sqlx::Error>>
    where
        E: Executor<'db, Database = AnyDb>;

    fn convert_result(result: Result<Self::Sqlx, sqlx::Error>) -> Result<Self::Rorm, Error>;
}

impl<Q: QueryByFuture> QueryType for Q {
    type Future<'result> = QueryFuture<'result, Q::Sqlx, Q::Rorm>;

    fn query<'post_query, E>(
        executor: E,
        query: String,
        values: Vec<Value<'post_query>>,
    ) -> Self::Future<'post_query>
    where
        E: Executor<'post_query, Database = AnyDb>,
    {
        let query_string = AliasableString::from_unique(query);
        let query: &str = &query_string;
        let mut query = sqlx::query(unsafe { std::mem::transmute(query) });
        for x in values {
            query = utils::bind_param(query, x);
        }
        QueryFuture {
            query_string,
            original: Q::create_future(executor, query),
            map: Q::convert_result,
        }
    }
}
// DO NOT modify this struct without careful thought!!!
pub(crate) struct QueryFuture<'q, Sqlx, Rorm> {
    #[allow(dead_code)] // it's not "dead", it is aliased before landing in this struct
    query_string: AliasableString,
    original: BoxFuture<'q, Result<Sqlx, sqlx::Error>>,
    map: fn(Result<Sqlx, sqlx::Error>) -> Result<Rorm, Error>,
}
impl<'q, Sqlx, Rorm> Future for QueryFuture<'q, Sqlx, Rorm> {
    type Output = Result<Rorm, Error>;

    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.original.as_mut().poll(cx) {
            Poll::Ready(output) => Poll::Ready((self.map)(output)),
            Poll::Pending => Poll::Pending,
        }
    }
}

type Offset = u64;

pub(crate) struct One;
impl GetLimitClause for One {
    type Input = Option<Offset>;

    fn get_limit_clause(offset: Self::Input) -> Option<LimitClause> {
        Some(LimitClause { limit: 1, offset })
    }
}
impl QueryByFuture for One {
    type Sqlx = sqlx::any::AnyRow;
    type Rorm = Row;

    fn create_future<'result, 'db: 'result, 'post_query: 'result, E>(
        executor: E,
        query: AnyQuery<'post_query>,
    ) -> BoxFuture<'result, Result<Self::Sqlx, sqlx::Error>>
    where
        E: Executor<'db, Database = AnyDb>,
    {
        executor.fetch_one(query)
    }

    fn convert_result(result: Result<Self::Sqlx, sqlx::Error>) -> Result<Self::Rorm, Error> {
        result.map(Row::from).map_err(Error::SqlxError)
    }
}

pub(crate) struct Optional;
impl GetLimitClause for Optional {
    type Input = Option<Offset>;

    fn get_limit_clause(offset: Self::Input) -> Option<LimitClause> {
        Some(LimitClause { limit: 1, offset })
    }
}
impl QueryByFuture for Optional {
    type Sqlx = Option<sqlx::any::AnyRow>;
    type Rorm = Option<Row>;

    fn create_future<'result, 'db: 'result, 'post_query: 'result, E>(
        executor: E,
        query: AnyQuery<'post_query>,
    ) -> BoxFuture<'result, Result<Self::Sqlx, sqlx::Error>>
    where
        E: Executor<'db, Database = AnyDb>,
    {
        executor.fetch_optional(query)
    }

    fn convert_result(result: Result<Self::Sqlx, sqlx::Error>) -> Result<Self::Rorm, Error> {
        result
            .map(|option| option.map(Row::from))
            .map_err(Error::SqlxError)
    }
}

pub(crate) struct All;
impl GetLimitClause for All {
    type Input = Option<LimitClause>;

    fn get_limit_clause(limit: Self::Input) -> Option<LimitClause> {
        limit
    }
}
impl QueryByFuture for All {
    type Sqlx = Vec<sqlx::any::AnyRow>;
    type Rorm = Vec<Row>;

    fn create_future<'result, 'db: 'result, 'post_query: 'result, E>(
        executor: E,
        query: AnyQuery<'post_query>,
    ) -> BoxFuture<'result, Result<Self::Sqlx, sqlx::Error>>
    where
        E: Executor<'db, Database = AnyDb>,
    {
        executor.fetch_all(query)
    }

    fn convert_result(result: Result<Self::Sqlx, sqlx::Error>) -> Result<Self::Rorm, Error> {
        result
            .map(|vector| vector.into_iter().map(Row::from).collect())
            .map_err(Error::SqlxError)
    }
}

pub(crate) struct Stream;
impl GetLimitClause for Stream {
    type Input = Option<LimitClause>;

    fn get_limit_clause(limit: Self::Input) -> Option<LimitClause> {
        limit
    }
}
impl QueryType for Stream {
    type Future<'result> = BoxStream<'result, Result<Row, Error>>;

    fn query<'post_query, E>(
        executor: E,
        query: String,
        values: Vec<Value<'post_query>>,
    ) -> Self::Future<'post_query>
    where
        E: Executor<'post_query, Database = AnyDb>,
    {
        QueryStream::build(query, values, executor).boxed()
    }
}

pub(crate) struct Nothing;
impl QueryByFuture for Nothing {
    type Sqlx = AnyQueryResult;
    type Rorm = ();

    fn create_future<'result, 'db: 'result, 'post_query: 'result, E>(
        executor: E,
        query: AnyQuery<'post_query>,
    ) -> BoxFuture<'result, Result<Self::Sqlx, sqlx::Error>>
    where
        E: Executor<'db, Database = AnyDb>,
    {
        executor.execute(query)
    }

    fn convert_result(result: Result<Self::Sqlx, sqlx::Error>) -> Result<Self::Rorm, Error> {
        result.map(|_| ()).map_err(Error::SqlxError)
    }
}
