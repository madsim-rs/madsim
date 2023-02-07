use crate::region::{Region, SigningRegion};
use crate::SigningService;
use aws_smithy_http::endpoint::{Endpoint, EndpointPrefix};
use std::error::Error;
use std::fmt::Debug;

#[derive(Clone, Debug)]
pub struct AwsEndpoint {
    endpoint: Endpoint,
    credential_scope: CredentialScope,
}

impl AwsEndpoint {
    pub fn new(endpoint: Endpoint, credential_scope: CredentialScope) -> AwsEndpoint {
        AwsEndpoint {
            endpoint,
            credential_scope,
        }
    }

    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    pub fn credential_scope(&self) -> &CredentialScope {
        &self.credential_scope
    }

    pub fn set_endpoint(&self, uri: &mut http::Uri, endpoint_prefix: Option<&EndpointPrefix>) {
        self.endpoint.set_endpoint(uri, endpoint_prefix);
    }
}

pub type BoxError = Box<dyn Error + Send + Sync + 'static>;

pub trait ResolveAwsEndpoint: Send + Sync + Debug {
    fn resolve_endpoint(&self, region: &Region) -> Result<AwsEndpoint, BoxError>;
}

#[derive(Clone, Default, Debug)]
pub struct CredentialScope {
    region: Option<SigningRegion>,
    service: Option<SigningService>,
}

impl CredentialScope {
    pub fn builder() -> credential_scope::Builder {
        credential_scope::Builder::default()
    }
}

pub mod credential_scope {
    use crate::endpoint::CredentialScope;
    use crate::region::SigningRegion;
    use crate::SigningService;

    #[derive(Debug, Default)]
    pub struct Builder {
        region: Option<SigningRegion>,
        service: Option<SigningService>,
    }

    impl Builder {
        pub fn region(mut self, region: impl Into<SigningRegion>) -> Self {
            self.region = Some(region.into());
            self
        }

        pub fn service(mut self, service: impl Into<SigningService>) -> Self {
            self.service = Some(service.into());
            self
        }

        pub fn build(self) -> CredentialScope {
            CredentialScope {
                region: self.region,
                service: self.service,
            }
        }
    }
}

impl CredentialScope {
    pub fn region(&self) -> Option<&SigningRegion> {
        self.region.as_ref()
    }

    pub fn service(&self) -> Option<&SigningService> {
        self.service.as_ref()
    }

    pub fn merge(&self, other: &CredentialScope) -> CredentialScope {
        CredentialScope {
            region: self.region.clone().or_else(|| other.region.clone()),
            service: self.service.clone().or_else(|| other.service.clone()),
        }
    }
}

impl ResolveAwsEndpoint for Endpoint {
    fn resolve_endpoint(&self, _region: &Region) -> Result<AwsEndpoint, BoxError> {
        Ok(AwsEndpoint {
            endpoint: self.clone(),
            credential_scope: Default::default(),
        })
    }
}
