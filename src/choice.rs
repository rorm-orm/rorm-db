//! Wrapper around string which is decodable as an enum

/// Wrapper around string which is decodable as an enum
pub struct Choice(pub String);

#[cfg(feature = "sqlx")]
impl sqlx::Type<sqlx::Any> for Choice {
    fn type_info() -> sqlx::any::AnyTypeInfo {
        unimplemented!(
            "This method should never be called according to a comment in `sqlx_core::any::type`"
        )
    }
    fn compatible(_ty: &sqlx::any::AnyTypeInfo) -> bool {
        // Sadly `sqlx::any::AnyTypeInfo`'s internals are all private,
        // so no proper check can be implemented.
        // If the type happens to be non compatible, decode will hopefully fail™.
        true
    }
}

#[cfg(feature = "sqlx")]
impl<'r> sqlx::Decode<'r, sqlx::Any> for Choice {
    fn decode(
        value: <sqlx::Any as sqlx::database::HasValueRef<'r>>::ValueRef,
    ) -> Result<Self, sqlx::error::BoxDynError> {
        <&'r str as sqlx::Decode<'r, sqlx::Any>>::decode(value)
            .map(|string| Choice(string.to_string()))
    }
}
