#[derive(Clone, Debug)]
pub(crate) enum AnyPool {
    Postgres(sqlx::Pool<sqlx::Postgres>),
    MySql(sqlx::Pool<sqlx::MySql>),
    Sqlite(sqlx::Pool<sqlx::Sqlite>),
}

impl AnyPool {
    pub(crate) async fn begin(&self) -> sqlx::Result<AnyTransaction> {
        match self {
            Self::Postgres(pool) => pool.begin().await.map(AnyTransaction::Postgres),
            Self::MySql(pool) => pool.begin().await.map(AnyTransaction::MySql),
            Self::Sqlite(pool) => pool.begin().await.map(AnyTransaction::Sqlite),
        }
    }

    pub(crate) fn query<'q>(&self, string: &'q str) -> AnyQuery<'q> {
        match self {
            AnyPool::Postgres(_) => AnyQuery::Postgres(sqlx::query(string)),
            AnyPool::MySql(_) => AnyQuery::MySql(sqlx::query(string)),
            AnyPool::Sqlite(_) => AnyQuery::Sqlite(sqlx::query(string)),
        }
    }
}

pub(crate) enum AnyTransaction {
    Postgres(sqlx::Transaction<'static, sqlx::Postgres>),
    MySql(sqlx::Transaction<'static, sqlx::MySql>),
    Sqlite(sqlx::Transaction<'static, sqlx::Sqlite>),
}

impl AnyTransaction {
    pub(crate) async fn commit(self) -> sqlx::Result<()> {
        match self {
            Self::Postgres(tx) => tx.commit().await,
            Self::MySql(tx) => tx.commit().await,
            Self::Sqlite(tx) => tx.commit().await,
        }
    }

    pub(crate) async fn rollback(self) -> sqlx::Result<()> {
        match self {
            Self::Postgres(tx) => tx.rollback().await,
            Self::MySql(tx) => tx.rollback().await,
            Self::Sqlite(tx) => tx.rollback().await,
        }
    }
}

pub(crate) enum AnyQuery<'q> {
    Postgres(sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>),
    MySql(sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments>),
    Sqlite(sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>),
}

impl<'q> AnyQuery<'q> {
    pub(crate) fn bind<T>(self, value: T) -> Self
    where
        T: 'q + Send + AnyEncode<'q> + AnyType,
    {
        match self {
            Self::Postgres(query) => Self::Postgres(query.bind(value)),
            Self::MySql(query) => Self::MySql(query.bind(value)),
            Self::Sqlite(query) => Self::Sqlite(query.bind(value)),
        }
    }
}

pub(crate) enum AnyRow {
    Postgres(sqlx::postgres::PgRow),
    MySql(sqlx::mysql::MySqlRow),
    Sqlite(sqlx::sqlite::SqliteRow),
}

/// Trait alias combining all [`Encode<'q, DB>`](sqlx::Encode)
pub(crate) trait AnyEncode<'q>:
    sqlx::Encode<'q, sqlx::Postgres> + sqlx::Encode<'q, sqlx::MySql> + sqlx::Encode<'q, sqlx::Sqlite>
{
}
impl<
        'q,
        T: sqlx::Encode<'q, sqlx::Postgres>
            + sqlx::Encode<'q, sqlx::MySql>
            + sqlx::Encode<'q, sqlx::Sqlite>,
    > AnyEncode<'q> for T
{
}

/// Trait alias combining all [`Decode<'r, DB>`](sqlx::Decode)
pub(crate) trait AnyDecode<'r>:
    sqlx::Decode<'r, sqlx::Postgres> + sqlx::Decode<'r, sqlx::MySql> + sqlx::Decode<'r, sqlx::Sqlite>
{
}
impl<
        'r,
        T: sqlx::Decode<'r, sqlx::Postgres>
            + sqlx::Decode<'r, sqlx::MySql>
            + sqlx::Decode<'r, sqlx::Sqlite>,
    > AnyDecode<'r> for T
{
}

/// Trait alias combining all [`Type<DB>`](sqlx::Type)
pub(crate) trait AnyType:
    sqlx::Type<sqlx::Postgres> + sqlx::Type<sqlx::MySql> + sqlx::Type<sqlx::Sqlite>
{
}
impl<T: sqlx::Type<sqlx::Postgres> + sqlx::Type<sqlx::MySql> + sqlx::Type<sqlx::Sqlite>> AnyType
    for T
{
}

/// Trait alias combining all [`ColumnIndex<DB>`](sqlx::ColumnIndex)
pub(crate) trait AnyColumnIndex:
    sqlx::ColumnIndex<sqlx::postgres::PgRow>
    + sqlx::ColumnIndex<sqlx::mysql::MySqlRow>
    + sqlx::ColumnIndex<sqlx::sqlite::SqliteRow>
{
}
impl<
        T: sqlx::ColumnIndex<sqlx::postgres::PgRow>
            + sqlx::ColumnIndex<sqlx::mysql::MySqlRow>
            + sqlx::ColumnIndex<sqlx::sqlite::SqliteRow>,
    > AnyColumnIndex for T
{
}
