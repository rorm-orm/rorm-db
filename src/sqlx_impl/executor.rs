use std::future::{ready, Ready};
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::future::{self, BoxFuture, FutureExt, TryFutureExt};
use futures::stream::{self, BoxStream, TryCollect, TryFilterMap, TryStreamExt};
use rorm_sql::value::Value;
use rorm_sql::DBImpl;

use crate::executor::{
    AffectedRows, All, Executor, Nothing, One, Optional, QueryStrategy, QueryStrategyResult, Stream,
};
use crate::internal::any::{AnyExecutor, AnyPool, AnyQueryResult, AnyRow, AnyTransaction};
use crate::transaction::{Transaction, TransactionGuard};
use crate::{Database, Error, Row};

impl<'executor> Executor<'executor> for &'executor mut Transaction {
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
        Q::execute(&mut self.0, query, values)
    }

    fn dialect(&self) -> DBImpl {
        match self.0 {
            #[cfg(feature = "postgres")]
            AnyTransaction::Postgres(_) => DBImpl::Postgres,
            #[cfg(feature = "mysql")]
            AnyTransaction::MySql(_) => DBImpl::MySQL,
            #[cfg(feature = "sqlite")]
            AnyTransaction::Sqlite(_) => DBImpl::SQLite,
        }
    }

    type EnsureTransactionFuture = Ready<Result<TransactionGuard<'executor>, Error>>;

    fn ensure_transaction(
        self,
    ) -> BoxFuture<'executor, Result<TransactionGuard<'executor>, Error>> {
        Box::pin(ready(Ok(TransactionGuard::Borrowed(self))))
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
        Q::execute(&self.0, query, values)
    }

    fn dialect(&self) -> DBImpl {
        match self.0 {
            #[cfg(feature = "postgres")]
            AnyPool::Postgres(_) => DBImpl::Postgres,
            #[cfg(feature = "mysql")]
            AnyPool::MySql(_) => DBImpl::MySQL,
            #[cfg(feature = "sqlite")]
            AnyPool::Sqlite(_) => DBImpl::SQLite,
        }
    }

    type EnsureTransactionFuture = BoxFuture<'executor, Result<TransactionGuard<'executor>, Error>>;

    fn ensure_transaction(
        self,
    ) -> BoxFuture<'executor, Result<TransactionGuard<'executor>, Error>> {
        Box::pin(async move { self.start_transaction().await.map(TransactionGuard::Owned) })
    }
}

pub trait QueryStrategyImpl: QueryStrategyResult {
    fn execute<'query, E>(
        executor: E,
        query: String,
        values: Vec<Value<'query>>,
    ) -> Self::Result<'query>
    where
        E: AnyExecutor<'query>;
}

type AnyEither = sqlx::Either<AnyQueryResult, AnyRow>;
type FetchMany<'a> = BoxStream<'a, Result<AnyEither, sqlx::Error>>;

pub type QueryFuture<T> = QueryWrapper<T>;
pub type QueryStream<T> = QueryWrapper<T>;
pub use query_wrapper::QueryWrapper;

/// Private module to contain the internals behind a sound api
mod query_wrapper {
    use std::pin::Pin;

    use rorm_sql::value::Value;

    use crate::internal::any::{AnyExecutor, AnyQuery};

    #[doc(hidden)]
    #[pin_project::pin_project]
    pub struct QueryWrapper<T> {
        #[pin]
        wrapped: T,
        #[allow(dead_code)] // is used via a reference inside T
        query_string: String,
    }

    impl<'query, T: 'query> QueryWrapper<T> {
        /// Basic constructor which only performs the unsafe lifetime extension to be tested by miri
        pub(crate) fn new_basic(string: String, wrapped: impl FnOnce(&'query str) -> T) -> Self {
            let slice: &str = string.as_str();

            // SAFETY: The heap allocation won't be dropped or moved
            //         until `wrapped` which contains this reference is dropped.
            let slice: &'query str = unsafe { std::mem::transmute(slice) };

            Self {
                query_string: string,
                wrapped: wrapped(slice),
            }
        }

        pub fn new<'data: 'query>(
            executor: impl AnyExecutor<'query>,
            query_string: String,
            values: Vec<Value<'data>>,
            execute: impl FnOnce(AnyQuery<'query>) -> T,
        ) -> Self {
            Self::new_basic(query_string, move |query_string| {
                let mut query = executor.query(query_string);
                for value in values {
                    crate::internal::utils::bind_param(&mut query, value);
                }
                execute(query)
            })
        }
    }

    impl<T> QueryWrapper<T> {
        /// Project a [`Pin`] onto the `wrapped` field
        pub fn project_wrapped(self: Pin<&mut Self>) -> Pin<&mut T> {
            self.project().wrapped
        }
    }
}

impl<F> future::Future for QueryFuture<F>
where
    F: future::Future,
{
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.project_wrapped().poll(cx)
    }
}
impl<S> stream::Stream for QueryStream<S>
where
    S: stream::Stream,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.project_wrapped().poll_next(cx)
    }
}

impl QueryStrategyResult for Nothing {
    type Result<'query> = QueryFuture<
        future::MapOk<
            TryCollect<
                stream::ErrInto<stream::MapOk<FetchMany<'query>, fn(AnyEither) -> ()>, Error>,
                Vec<()>,
            >,
            fn(Vec<()>) -> (),
        >,
    >;
}

impl QueryStrategyImpl for Nothing {
    fn execute<'query, E>(
        executor: E,
        query: String,
        values: Vec<Value<'query>>,
    ) -> Self::Result<'query>
    where
        E: AnyExecutor<'query>,
    {
        fn dump<T>(_: T) {}
        let dump_either: fn(AnyEither) -> () = dump;
        let dump_vec: fn(Vec<()>) -> () = dump;
        QueryFuture::new(executor, query, values, |query| {
            query
                .fetch_many()
                .map_ok(dump_either)
                .err_into()
                .try_collect()
                .map_ok(dump_vec)
        })
    }
}

impl QueryStrategyResult for AffectedRows {
    type Result<'query> = QueryFuture<BoxFuture<'query, Result<u64, Error>>>;
}
impl QueryStrategyImpl for AffectedRows {
    fn execute<'query, E>(
        executor: E,
        query: String,
        values: Vec<Value<'query>>,
    ) -> Self::Result<'query>
    where
        E: AnyExecutor<'query>,
    {
        QueryFuture::new(executor, query, values, |query| {
            (async move { Ok(query.fetch_affected_rows().await?) }).boxed()
        })
    }
}

impl QueryStrategyResult for One {
    type Result<'query> = QueryFuture<BoxFuture<'query, Result<Row, Error>>>;
}
impl QueryStrategyImpl for One {
    fn execute<'query, E>(
        executor: E,
        query: String,
        values: Vec<Value<'query>>,
    ) -> Self::Result<'query>
    where
        E: AnyExecutor<'query>,
    {
        QueryFuture::new(executor, query, values, |query| {
            (async move {
                Ok(Row(query
                    .fetch_optional()
                    .await?
                    .ok_or(sqlx::Error::RowNotFound)?))
            })
            .boxed()
        })
    }
}

impl QueryStrategyResult for Optional {
    type Result<'query> = QueryFuture<BoxFuture<'query, Result<Option<Row>, Error>>>;
}
impl QueryStrategyImpl for Optional {
    fn execute<'query, E>(
        executor: E,
        query: String,
        values: Vec<Value<'query>>,
    ) -> Self::Result<'query>
    where
        E: AnyExecutor<'query>,
    {
        QueryFuture::new(executor, query, values, |query| {
            (async move { Ok(query.fetch_optional().await?.map(Row)) }).boxed()
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

impl QueryStrategyResult for All {
    type Result<'query> = QueryFuture<BoxFuture<'query, Result<Vec<Row>, Error>>>;
}
impl QueryStrategyImpl for All {
    fn execute<'query, E>(
        executor: E,
        query: String,
        values: Vec<Value<'query>>,
    ) -> Self::Result<'query>
    where
        E: AnyExecutor<'query>,
    {
        QueryFuture::new(executor, query, values, |query| {
            (async move { Ok(query.fetch_all().await?.into_iter().map(Row).collect()) }).boxed()
        })
    }
}

impl QueryStrategyResult for Stream {
    type Result<'query> = QueryStream<
        stream::ErrInto<
            TryFilterMap<
                FetchMany<'query>,
                Ready<Result<Option<Row>, sqlx::Error>>,
                fn(AnyEither) -> Ready<Result<Option<Row>, sqlx::Error>>,
            >,
            Error,
        >,
    >;
}
impl QueryStrategyImpl for Stream {
    fn execute<'query, E>(
        executor: E,
        query: String,
        values: Vec<Value<'query>>,
    ) -> Self::Result<'query>
    where
        E: AnyExecutor<'query>,
    {
        QueryStream::new(executor, query, values, |query| {
            query.fetch_many().try_filter_map(TRY_FILTER_MAP).err_into()
        })
    }
}

#[cfg(test)]
mod test {
    use crate::internal::executor::QueryWrapper;

    /// Run this test with miri
    ///
    /// If the drop order of [`QueryWrapper`]'s fields is incorrect,
    /// miri will complain about a use-after-free.
    #[test]
    fn test_drop_order() {
        struct BorrowStr<'a>(&'a str);
        impl<'a> Drop for BorrowStr<'a> {
            fn drop(&mut self) {
                // Use the borrowed string.
                // If it were already dropped, miri would detect it.
                println!("{}", self.0);
            }
        }
        let _w = QueryWrapper::new_basic(format!("Hello World"), BorrowStr);
    }
}
