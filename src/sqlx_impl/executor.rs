use std::future::Ready;
use std::pin::Pin;
use std::task::{Context, Poll};

use aliasable::string::AliasableString;
use futures::future::{self, BoxFuture, FutureExt, TryFutureExt};
use futures::stream::{self, BoxStream, ErrInto, TryCollect, TryFilterMap, TryStreamExt};
use rorm_sql::value::Value;

use crate::executor::{All, Executor, Nothing, One, Optional, QueryStrategy, Stream};
use crate::transaction::Transaction;
use crate::{utils, Database, Error, Row};

impl<'executor> Executor<'executor> for &'executor mut Transaction<'_> {
    fn execute<'data, 'result, Q>(
        self,
        query: String,
        values: Vec<Value<'data>>,
    ) -> Q::Result<'result>
    where
        'executor: 'result,
        'data: 'result,
        Q: QueryStrategy,
    {
        Q::execute(&mut self.tx, query, values)
    }
}

impl<'executor> Executor<'executor> for &'executor Database {
    fn execute<'data, 'result, Q>(
        self,
        query: String,
        values: Vec<Value<'data>>,
    ) -> Q::Result<'result>
    where
        'executor: 'result,
        'data: 'result,
        Q: QueryStrategy,
    {
        Q::execute(&self.pool, query, values)
    }
}

pub trait QueryStrategyImpl {
    type Result<'a>;

    fn execute<'executor, 'data, 'result, E>(
        executor: E,
        query: String,
        values: Vec<Value<'data>>,
    ) -> Self::Result<'result>
    where
        'executor: 'result,
        'data: 'result,
        E: sqlx::Executor<'executor, Database = sqlx::Any>;
}

type AnyQuery<'q> = sqlx::query::Query<'q, sqlx::Any, sqlx::any::AnyArguments<'q>>;
type AnyEither = sqlx::Either<sqlx::any::AnyQueryResult, sqlx::any::AnyRow>;
type FetchMany<'a> = BoxStream<'a, Result<AnyEither, sqlx::Error>>;
type FetchOptional<'a> = BoxFuture<'a, Result<Option<sqlx::any::AnyRow>, sqlx::Error>>;

pub type QueryFuture<T> = QueryWrapper<T>;

pub type QueryStream<T> = QueryWrapper<T>;

#[doc(hidden)]
#[pin_project::pin_project]
pub struct QueryWrapper<T> {
    #[allow(dead_code)]
    query_string: AliasableString,
    #[pin]
    wrapped: T,
}
impl<T> QueryWrapper<T> {
    fn new<'executor, 'data>(
        query: String,
        values: Vec<Value<'data>>,
        execute: impl FnOnce(AnyQuery<'data>) -> T,
    ) -> Self {
        let query_string = AliasableString::from_unique(query);
        let query: &str = &query_string;
        let query: &'data str = unsafe { std::mem::transmute(query) };
        let mut query = sqlx::query(query);
        for x in values {
            query = utils::bind_param(query, x);
        }
        Self {
            query_string,
            wrapped: execute(query),
        }
    }
}
impl<F> future::Future for QueryFuture<F>
where
    F: future::Future,
{
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.project().wrapped.poll(cx)
    }
}
impl<S> stream::Stream for QueryStream<S>
where
    S: stream::Stream,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.project().wrapped.poll_next(cx)
    }
}

impl QueryStrategyImpl for Nothing {
    type Result<'result> = QueryFuture<
        future::MapOk<
            TryCollect<
                stream::ErrInto<stream::MapOk<FetchMany<'result>, fn(AnyEither) -> ()>, Error>,
                Vec<()>,
            >,
            fn(Vec<()>) -> (),
        >,
    >;

    fn execute<'executor, 'data, 'result, E>(
        executor: E,
        query: String,
        values: Vec<Value<'data>>,
    ) -> Self::Result<'result>
    where
        'executor: 'result,
        'data: 'result,
        E: sqlx::Executor<'executor, Database = sqlx::Any>,
    {
        fn dump<T>(_: T) {}
        let dump_either: fn(AnyEither) -> () = dump;
        let dump_vec: fn(Vec<()>) -> () = dump;
        QueryFuture::new(query, values, |query| {
            executor
                .fetch_many::<'result, 'data, AnyQuery<'data>>(query)
                .map_ok(dump_either)
                .err_into()
                .try_collect()
                .map_ok(dump_vec)
        })
    }
}

impl QueryStrategyImpl for One {
    type Result<'result> = QueryFuture<
        future::ErrInto<
            future::Map<
                FetchOptional<'result>,
                fn(Result<Option<sqlx::any::AnyRow>, sqlx::Error>) -> Result<Row, sqlx::Error>,
            >,
            Error,
        >,
    >;

    fn execute<'executor, 'data, 'result, E>(
        executor: E,
        query: String,
        values: Vec<Value<'data>>,
    ) -> Self::Result<'result>
    where
        'executor: 'result,
        'data: 'result,
        E: sqlx::Executor<'executor, Database = sqlx::Any>,
    {
        fn convert(
            result: Result<Option<sqlx::any::AnyRow>, sqlx::Error>,
        ) -> Result<Row, sqlx::Error> {
            result.and_then(|row| row.map(Row).ok_or(sqlx::Error::RowNotFound))
        }
        let convert: fn(
            Result<Option<sqlx::any::AnyRow>, sqlx::Error>,
        ) -> Result<Row, sqlx::Error> = convert;
        QueryFuture::new(query, values, |query| {
            executor.fetch_optional(query).map(convert).err_into()
        })
    }
}

impl QueryStrategyImpl for Optional {
    type Result<'result> = QueryFuture<
        future::ErrInto<
            future::MapOk<FetchOptional<'result>, fn(Option<sqlx::any::AnyRow>) -> Option<Row>>,
            Error,
        >,
    >;

    fn execute<'executor, 'data, 'result, E>(
        executor: E,
        query: String,
        values: Vec<Value<'data>>,
    ) -> Self::Result<'result>
    where
        'executor: 'result,
        'data: 'result,
        E: sqlx::Executor<'executor, Database = sqlx::Any>,
    {
        fn convert(option: Option<sqlx::any::AnyRow>) -> Option<Row> {
            option.map(Row)
        }
        let convert: fn(Option<sqlx::any::AnyRow>) -> Option<Row> = convert;
        QueryFuture::new(query, values, |query| {
            executor.fetch_optional(query).map_ok(convert).err_into()
        })
    }
}

/// Function used by [All] and [Stream] in [try_filter_map](TryStreamExt::try_filter_map).
static TRY_FILTER_MAP: fn(AnyEither) -> Ready<Result<Option<Row>, sqlx::Error>> = {
    fn convert(either: AnyEither) -> Ready<Result<Option<Row>, sqlx::Error>> {
        std::future::ready(Ok(match either {
            AnyEither::Left(_) => None,
            AnyEither::Right(row) => Some(Row(row)),
        }))
    }
    convert
};

impl QueryStrategyImpl for All {
    type Result<'result> = QueryFuture<
        TryCollect<
            ErrInto<
                TryFilterMap<
                    FetchMany<'result>,
                    Ready<Result<Option<Row>, sqlx::Error>>,
                    fn(AnyEither) -> Ready<Result<Option<Row>, sqlx::Error>>,
                >,
                Error,
            >,
            Vec<Row>,
        >,
    >;

    fn execute<'executor, 'data, 'result, E>(
        executor: E,
        query: String,
        values: Vec<Value<'data>>,
    ) -> Self::Result<'result>
    where
        'executor: 'result,
        'data: 'result,
        E: sqlx::Executor<'executor, Database = sqlx::Any>,
    {
        QueryFuture::new(query, values, |query| {
            executor
                .fetch_many(query)
                .try_filter_map(TRY_FILTER_MAP)
                .err_into()
                .try_collect()
        })
    }
}

impl QueryStrategyImpl for Stream {
    type Result<'result> = QueryStream<
        ErrInto<
            TryFilterMap<
                FetchMany<'result>,
                Ready<Result<Option<Row>, sqlx::Error>>,
                fn(AnyEither) -> Ready<Result<Option<Row>, sqlx::Error>>,
            >,
            Error,
        >,
    >;

    fn execute<'executor, 'data, 'result, E>(
        executor: E,
        query: String,
        values: Vec<Value<'data>>,
    ) -> Self::Result<'result>
    where
        'executor: 'result,
        'data: 'result,
        E: sqlx::Executor<'executor, Database = sqlx::Any>,
    {
        QueryStream::new(query, values, |query| {
            executor
                .fetch_many(query)
                .try_filter_map(TRY_FILTER_MAP)
                .err_into()
        })
    }
}
