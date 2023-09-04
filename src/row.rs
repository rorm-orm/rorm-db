//! This module defines a wrapper for sqlx's AnyRow

use crate::error::Error;
use crate::internal;

/// Represents a single row from the database.
pub struct Row(pub(crate) internal::row::Impl);

impl Row {
    /// Index into the database row and decode a single value.
    ///
    /// A string index can be used to access a column by name
    /// and a `usize` index can be used to access a column by position.
    pub fn get<'r, T, I>(&'r self, index: I) -> Result<T, Error>
    where
        T: Decode<'r>,
        I: RowIndex,
    {
        internal::row::get(self, index)
    }
}

impl From<internal::row::Impl> for Row {
    fn from(row: internal::row::Impl) -> Self {
        Row(row)
    }
}

/// Something which can be decoded from a [`Row`]'s cell.
#[cfg(feature = "sqlx")]
pub trait Decode<'r>: internal::any::AnyType + internal::any::AnyDecode<'r> {}
/// Something which can be decoded from a [`Row`]'s cell.
#[cfg(not(feature = "sqlx"))]
pub trait Decode<'r> {}

/// Something which can be used to index a [`Row`]'s cells.
#[cfg(feature = "sqlx")]
pub trait RowIndex: internal::any::AnyColumnIndex {}
/// Something which can be used to index a [`Row`]'s cells.
#[cfg(not(feature = "sqlx"))]
pub trait RowIndex {}

/// Something which can be decoded from a [`Row`]'s cell without borrowing.
pub trait DecodeOwned: for<'r> Decode<'r> {}
impl<T: for<'r> Decode<'r>> DecodeOwned for T {}

#[cfg(feature = "sqlx")]
const _: () = {
    impl<'r, T: internal::any::AnyType + internal::any::AnyDecode<'r>> Decode<'r> for T {}
    impl<T: internal::any::AnyColumnIndex> RowIndex for T {}
};
#[cfg(not(feature = "sqlx"))]
const _: () = {
    impl<'r, T> Decode<'r> for T {}
    impl RowIndex for usize {}
    impl RowIndex for &str {}
};
