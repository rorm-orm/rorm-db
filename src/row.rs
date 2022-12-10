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

/// Something which can be decoded from a [row](Row)'s cell.
#[cfg(feature = "sqlx")]
pub trait Decode<'r>: sqlx::Type<sqlx::Any> + sqlx::Decode<'r, sqlx::Any> {}
/// Something which can be decoded from a [row](Row)'s cell.
#[cfg(not(feature = "sqlx"))]
pub trait Decode<'r> {}

/// Something which can be used to index a [row](Row)'s cells.
#[cfg(feature = "sqlx")]
pub trait RowIndex: sqlx::ColumnIndex<sqlx::any::AnyRow> {}
/// Something which can be used to index a [row](Row)'s cells.
#[cfg(not(feature = "sqlx"))]
pub trait RowIndex {}

/// Something which can be decoded from a [row](Row)'s cell without borrowing.
pub trait DecodeOwned: for<'r> Decode<'r> {}
impl<T: for<'r> Decode<'r>> DecodeOwned for T {}

#[cfg(feature = "sqlx")]
const _: () = {
    impl<'r, T: sqlx::Type<sqlx::Any> + sqlx::Decode<'r, sqlx::Any>> Decode<'r> for T {}
    impl<T: sqlx::ColumnIndex<sqlx::any::AnyRow>> RowIndex for T {}
};
#[cfg(not(feature = "sqlx"))]
const _: () = {
    impl<'r, T> Decode<'r> for T {}
    impl RowIndex for usize {}
    impl RowIndex for &str {}
};

/// Something which can be decoded from a [row](Row).
///
/// Auto-implemented for tuples of size 8 or less.
pub trait FromRow: Sized {
    /// Try decoding a [row](Row) into `Self`.
    fn from_row(row: Row) -> Result<Self, Error>;
}

macro_rules! impl_from_row {
    ($($generic:ident@$index:literal),+) => {
        impl<$($generic),+> FromRow for ($($generic,)+)
        where
            $(
                $generic: DecodeOwned,
            )+
        {
            fn from_row(row: Row) -> Result<Self, Error> {
                Ok((
                    $(
                        row.get::<$generic, usize>($index)?,
                    )+
                ))
            }
        }
    };
}
impl_from_row!(A@0);
impl_from_row!(A@0, B@1);
impl_from_row!(A@0, B@1, C@2);
impl_from_row!(A@0, B@1, C@2, D@3);
impl_from_row!(A@0, B@1, C@2, D@3, E@4);
impl_from_row!(A@0, B@1, C@2, D@3, E@4, F@5);
impl_from_row!(A@0, B@1, C@2, D@3, E@4, F@5, G@6);
impl_from_row!(A@0, B@1, C@2, D@3, E@4, F@5, G@6, H@7);
