use crate::row::{Decode, RowIndex};
use crate::{Error, Row};

pub(crate) type Impl = sqlx::any::AnyRow;

/// Implementation of [Row::get]
pub(crate) fn get<'r, T, I>(row: &'r Row, index: I) -> Result<T, Error>
where
    T: Decode<'r>,
    I: RowIndex,
{
    <sqlx::any::AnyRow as sqlx::Row>::try_get(&row.0, index).map_err(Error::SqlxError)
}
