use std::{any, fmt, str};
use tonic::metadata::errors::InvalidMetadataValue;
use tonic::metadata::{Ascii, MetadataValue};
use tonic::service::Interceptor;
use tonic::{Request, Status};

const X_GOOG_API_KEY: &str = "x-goog-api-key";

pub struct Authorisation {
    api_key: MetadataValue<Ascii>,
}

impl fmt::Debug for Authorisation {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "{}(<redacted>)", any::type_name::<Self>())
    }
}

impl str::FromStr for Authorisation {
    type Err = InvalidMetadataValue;

    fn from_str(api_key: &str) -> Result<Self, Self::Err> {
        api_key.parse().map(|api_key| Self { api_key })
    }
}

impl Interceptor for Authorisation {
    fn call(&mut self, mut request: Request<()>) -> Result<Request<()>, Status> {
        request
            .metadata_mut()
            .insert(X_GOOG_API_KEY, self.api_key.clone());

        Ok(request)
    }
}
