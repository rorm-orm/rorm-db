//! Utility functions

use rorm_sql::value::{NullType, Value};
use sqlx::types::chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use sqlx::types::time::{Date, OffsetDateTime, PrimitiveDateTime, Time};
use sqlx::types::{Json, Uuid};

use super::any::AnyQuery;

/// This helper method is used to bind ConditionValues to the query.
pub fn bind_param<'post_query, 'query>(query: &mut AnyQuery<'query>, param: Value<'post_query>)
where
    'post_query: 'query,
{
    match param {
        Value::String(x) => query.bind(x),
        Value::I64(x) => query.bind(x),
        Value::I32(x) => query.bind(x),
        Value::I16(x) => query.bind(x),
        Value::Bool(x) => query.bind(x),
        Value::F32(x) => query.bind(x),
        Value::F64(x) => query.bind(x),
        Value::Binary(x) => query.bind(x),
        Value::Ident(_) => {}
        Value::Column { .. } => {}
        Value::Choice(_) => {}

        Value::ChronoNaiveDate(x) => query.bind(x),
        Value::ChronoNaiveTime(x) => query.bind(x),
        Value::ChronoNaiveDateTime(x) => query.bind(x),
        Value::ChronoDateTime(x) => query.bind(x),

        Value::TimeDate(x) => query.bind(x),
        Value::TimeTime(x) => query.bind(x),
        Value::TimeOffsetDateTime(x) => query.bind(x),
        Value::TimePrimitiveDateTime(x) => query.bind(x),

        Value::Uuid(x) => query.bind(x),
        Value::UuidHyphenated(x) => query.bind(x.hyphenated().to_string()),
        Value::UuidSimple(x) => query.bind(x.simple().to_string()),

        Value::JsonValue(x) => query.bind(Json(x)),

        Value::Null(null_type) => match null_type {
            NullType::String => query.bind(None::<&str>),
            NullType::I64 => query.bind(None::<i64>),
            NullType::I32 => query.bind(None::<i32>),
            NullType::I16 => query.bind(None::<i16>),
            NullType::Bool => query.bind(None::<bool>),
            NullType::F64 => query.bind(None::<f64>),
            NullType::F32 => query.bind(None::<f32>),
            NullType::Binary => query.bind(None::<&[u8]>),
            NullType::Choice => {}

            NullType::ChronoNaiveTime => query.bind(None::<NaiveTime>),
            NullType::ChronoNaiveDate => query.bind(None::<NaiveDate>),
            NullType::ChronoNaiveDateTime => query.bind(None::<NaiveDateTime>),
            NullType::ChronoDateTime => query.bind(None::<DateTime<Utc>>),

            NullType::TimeDate => query.bind(None::<Date>),
            NullType::TimeTime => query.bind(None::<Time>),
            NullType::TimeOffsetDateTime => query.bind(None::<OffsetDateTime>),
            NullType::TimePrimitiveDateTime => query.bind(None::<PrimitiveDateTime>),

            NullType::Uuid => query.bind(None::<Uuid>),
            NullType::UuidHyphenated => query.bind(None::<String>),
            NullType::UuidSimple => query.bind(None::<String>),

            NullType::JsonValue => query.bind(None::<Json<()>>),
        },
    }
}
