use sqlx::Row as SqlxRowTrait;

use crate::internal::any::AnyRow;
use crate::row::{Decode, RowIndex};
use crate::{Error, Row};

pub(crate) type Impl = AnyRow;

/// Implementation of [Row::get]
pub(crate) fn get<'r, T, I>(row: &'r Row, index: I) -> Result<T, Error>
where
    T: Decode<'r>,
    I: RowIndex,
{
    let result = match &row.0 {
        #[cfg(feature = "postgres")]
        AnyRow::Postgres(row) => row.try_get(index),
        #[cfg(feature = "mysql")]
        AnyRow::MySql(row) => row.try_get(index),
        #[cfg(feature = "sqlite")]
        AnyRow::Sqlite(row) => row.try_get(index),
    };
    result.map_err(Error::SqlxError)
}
