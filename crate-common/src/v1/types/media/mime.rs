use core::ops::Deref;
use std::{fmt::Display, str::FromStr};

use mediatype::{MediaTypeBuf, MediaTypeError};
use serde::{Deserialize, Serialize};

/// A mime/media type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mime(MediaTypeBuf);

impl FromStr for Mime {
    type Err = MediaTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.parse()?))
    }
}

impl Deref for Mime {
    type Target = MediaTypeBuf;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for Mime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(feature = "utoipa")]
mod schema {
    use utoipa::{
        openapi::{RefOr, Schema},
        schema, PartialSchema, ToSchema,
    };

    use super::Mime;

    impl PartialSchema for Mime {
        fn schema() -> RefOr<Schema> {
            let schema = schema!(String)
                .title(Some("Mime"))
                .description(Some("a mime/media type"))
                .build();
            RefOr::T(Schema::Object(schema))
        }
    }

    impl ToSchema for Mime {}
}
