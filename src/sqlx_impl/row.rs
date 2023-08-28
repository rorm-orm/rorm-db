use crate::internal::any::AnyRow;
use crate::row::{Decode, RowIndex};
use crate::{Error, Row};
use sqlx::Row as SqlxRow;

pub(crate) type Impl = AnyRow;

/// Implementation of [Row::get]
pub(crate) fn get<'r, T, I>(row: &'r Row, index: I) -> Result<T, Error>
where
    T: Decode<'r>,
    I: RowIndex,
{
    let result = match &row.0 {
        AnyRow::Postgres(row) => row.try_get(index),
        AnyRow::MySql(row) => row.try_get(index),
        AnyRow::Sqlite(row) => row.try_get(index),
    };
    result.map_err(Error::SqlxError)
}
