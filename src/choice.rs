//! Wrapper around string which is decodable as an enum

use std::marker::PhantomData;

/// Wrapper around string which is decodable as an enum
///
/// Postgres creates a new type for enums with a unique name.
/// Therefore, `Choice` is generic over a ZST which implements [`ChoiceName`] to provide the postgres name.
///
/// If the generic is not provided, `Choice` won't be usable with postgres
pub struct Choice<N = ()>(pub String, pub PhantomData<N>);

/// Trait to associate a postgres type name with [`Choice`]
pub trait ChoiceName {
    /// The sql name to use for postgres
    const NAME: &'static str;
}

#[cfg(feature = "sqlx")]
const _: () = {
    use sqlx::database::{Database, HasValueRef};
    use sqlx::error::BoxDynError;
    use sqlx::{Decode, Type};

    #[cfg(feature = "postgres")]
    const _: () = {
        use sqlx::Postgres;
        impl<N: ChoiceName> Type<Postgres> for Choice<N> {
            fn type_info() -> <Postgres as Database>::TypeInfo {
                sqlx::postgres::PgTypeInfo::with_name(N::NAME)
            }
            fn compatible(ty: &<Postgres as Database>::TypeInfo) -> bool {
                *ty == sqlx::postgres::PgTypeInfo::with_name(N::NAME)
                    || <str as Type<Postgres>>::compatible(ty)
            }
        }
        impl<'r, N: ChoiceName> Decode<'r, Postgres> for Choice<N> {
            fn decode(value: <Postgres as HasValueRef<'r>>::ValueRef) -> Result<Self, BoxDynError> {
                <String as Decode<'r, Postgres>>::decode(value)
                    .map(|string| Self(string, PhantomData))
            }
        }
    };

    #[cfg(feature = "mysql")]
    const _: () = {
        use sqlx::MySql;
        impl<N> Type<MySql> for Choice<N> {
            fn type_info() -> <MySql as Database>::TypeInfo {
                <str as Type<MySql>>::type_info()
            }
            fn compatible(ty: &<MySql as Database>::TypeInfo) -> bool {
                <str as Type<MySql>>::compatible(ty)
            }
        }
        impl<'r, N> Decode<'r, MySql> for Choice<N> {
            fn decode(value: <MySql as HasValueRef<'r>>::ValueRef) -> Result<Self, BoxDynError> {
                <String as Decode<'r, MySql>>::decode(value).map(|string| Self(string, PhantomData))
            }
        }
    };

    #[cfg(feature = "sqlite")]
    const _: () = {
        use sqlx::Sqlite;
        impl<N> Type<Sqlite> for Choice<N> {
            fn type_info() -> <Sqlite as Database>::TypeInfo {
                <str as Type<Sqlite>>::type_info()
            }
            fn compatible(ty: &<Sqlite as Database>::TypeInfo) -> bool {
                <str as Type<Sqlite>>::compatible(ty)
            }
        }
        impl<'r, N> Decode<'r, Sqlite> for Choice<N> {
            fn decode(value: <Sqlite as HasValueRef<'r>>::ValueRef) -> Result<Self, BoxDynError> {
                <String as Decode<'r, Sqlite>>::decode(value)
                    .map(|string| Self(string, PhantomData))
            }
        }
    };
};
