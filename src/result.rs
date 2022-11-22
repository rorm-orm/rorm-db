use std::pin::Pin;
use std::task::{Context, Poll};

use futures::stream::BoxStream;
use futures::Stream;
use rorm_sql::value;
use sqlx::any::AnyRow;
use sqlx::Executor;

use crate::error::Error;
use crate::row::Row;
use crate::utils;

#[ouroboros::self_referencing]
pub struct QueryStream<'post_query> {
    pub(crate) query_str: String,
    pub(crate) bind_params: Vec<value::Value<'post_query>>,
    #[borrows(query_str, bind_params)]
    #[not_covariant]
    pub(crate) stream: BoxStream<'this, Result<AnyRow, sqlx::Error>>,
}

impl<'post_query> QueryStream<'post_query> {
    pub(crate) fn build<E>(
        stmt: String,
        bind_params: Vec<value::Value<'post_query>>,
        executor: E,
    ) -> QueryStream<'post_query>
    where
        E: Executor<'post_query, Database = sqlx::Any>,
    {
        QueryStream::new(stmt, bind_params, |stmt, bind_params| {
            let mut tmp = sqlx::query(stmt);

            for x in bind_params {
                tmp = utils::bind_param(tmp, *x);
            }

            return tmp.fetch(executor);
        })
    }

    #[allow(dead_code)]
    /**
    This function only exists to suppress the unused code warning of the orouboros
    */
    fn _suppress_warnings(&self) {
        self.borrow_query_str();
        self.borrow_bind_params();
    }
}

impl Stream for QueryStream<'_> {
    type Item = Result<Row, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.with_stream_mut(|x| {
            x.as_mut()
                .poll_next(cx)
                .map(|option| option.map(|result| result.map(Row::from).map_err(Error::SqlxError)))
        })
    }
}
