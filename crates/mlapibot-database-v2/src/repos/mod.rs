macro_rules! impl_sql_fwd {
    ($name:ident as $type:ident) => {
        impl<'a> postgres_types::FromSql<'a> for $name {
            fn from_sql(
                ty: &postgres_types::Type,
                raw: &'a [u8],
            ) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
                <$type as postgres_types::FromSql>::from_sql(ty, raw).map(Self)
            }

            fn accepts(ty: &postgres_types::Type) -> bool {
                <$type as postgres_types::FromSql>::accepts(ty)
            }

            fn from_sql_null(
                ty: &postgres_types::Type,
            ) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
                <$type as postgres_types::FromSql>::from_sql_null(ty).map(Self)
            }

            fn from_sql_nullable(
                ty: &postgres_types::Type,
                raw: Option<&'a [u8]>,
            ) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
                <$type as postgres_types::FromSql>::from_sql_nullable(ty, raw).map(Self)
            }
        }

        impl postgres_types::ToSql for $name {
            fn to_sql(
                &self,
                ty: &postgres_types::Type,
                out: &mut tokio_postgres::types::private::BytesMut,
            ) -> Result<postgres_types::IsNull, Box<dyn std::error::Error + Sync + Send>>
            where
                Self: Sized,
            {
                <$type as postgres_types::ToSql>::to_sql(&self.0, ty, out)
            }

            fn accepts(ty: &postgres_types::Type) -> bool
            where
                Self: Sized,
            {
                <$type as postgres_types::ToSql>::accepts(ty)
            }

            fn to_sql_checked(
                &self,
                ty: &postgres_types::Type,
                out: &mut tokio_postgres::types::private::BytesMut,
            ) -> Result<postgres_types::IsNull, Box<dyn std::error::Error + Sync + Send>> {
                <$type as postgres_types::ToSql>::to_sql_checked(&self.0, ty, out)
            }

            fn encode_format(&self, ty: &postgres_types::Type) -> postgres_types::Format {
                <$type as postgres_types::ToSql>::encode_format(&self.0, ty)
            }
        }
    };
}

macro_rules! make_newtype_id {
    ($($name:ident),* $(,)?) => {
        $(
            #[derive(
                Debug, Clone, Copy,
                PartialEq, Eq, PartialOrd, Ord, Hash,
                serde::Serialize, serde::Deserialize
            )]
            #[serde(transparent)]
            pub struct $name(i32);

            impl_sql_fwd!($name as i32);
        )*
    };
}

pub(self) use impl_sql_fwd;
pub(self) use make_newtype_id;

pub mod incidents;
pub mod monitor;
pub mod staff_replies;
pub mod subreddits;
