//! Wrapper around string which is decodable as an enum

/// Wrapper around string which is decodable as an enum
pub struct Choice(pub String);

#[cfg(feature = "sqlx")]
const _: () = {
    use sqlx::database::{Database, HasValueRef};
    use sqlx::error::BoxDynError;
    use sqlx::{Decode, MySql, Postgres, Sqlite, Type};
    impl Type<Postgres> for Choice {
        fn type_info() -> <Postgres as Database>::TypeInfo {
            <str as Type<Postgres>>::type_info()
        }
        fn compatible(ty: &<Postgres as Database>::TypeInfo) -> bool {
            <str as Type<Postgres>>::compatible(ty)
        }
    }
    impl<'r> Decode<'r, Postgres> for Choice {
        fn decode(value: <Postgres as HasValueRef<'r>>::ValueRef) -> Result<Self, BoxDynError> {
            <String as Decode<'r, Postgres>>::decode(value).map(Self)
        }
    }

    impl Type<MySql> for Choice {
        fn type_info() -> <MySql as Database>::TypeInfo {
            <str as Type<MySql>>::type_info()
        }
        fn compatible(ty: &<MySql as Database>::TypeInfo) -> bool {
            <str as Type<MySql>>::compatible(ty)
        }
    }
    impl<'r> Decode<'r, MySql> for Choice {
        fn decode(value: <MySql as HasValueRef<'r>>::ValueRef) -> Result<Self, BoxDynError> {
            <String as Decode<'r, MySql>>::decode(value).map(Self)
        }
    }

    impl Type<Sqlite> for Choice {
        fn type_info() -> <Sqlite as Database>::TypeInfo {
            <str as Type<Sqlite>>::type_info()
        }
        fn compatible(ty: &<Sqlite as Database>::TypeInfo) -> bool {
            <str as Type<Sqlite>>::compatible(ty)
        }
    }
    impl<'r> Decode<'r, Sqlite> for Choice {
        fn decode(value: <Sqlite as HasValueRef<'r>>::ValueRef) -> Result<Self, BoxDynError> {
            <String as Decode<'r, Sqlite>>::decode(value).map(Self)
        }
    }
};
