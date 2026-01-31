#[derive(Debug, Clone, Default)]
pub struct S3Config {
    pub region: Option<String>,
    pub credential_source: Option<String>,
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
}

impl S3Config {
    pub fn builder() -> S3ConfigBuilder {
        S3ConfigBuilder::default()
    }
}

#[derive(Debug, Clone, Default)]
pub struct S3ConfigBuilder {
    region: Option<String>,
    credential_source: Option<String>,
    access_key_id: Option<String>,
    secret_access_key: Option<String>,
}

impl S3ConfigBuilder {
    pub fn region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }

    pub fn credential_source(mut self, source: impl Into<String>) -> Self {
        self.credential_source = Some(source.into());
        self
    }

    pub fn access_key_id(mut self, key: impl Into<String>) -> Self {
        self.access_key_id = Some(key.into());
        self
    }

    pub fn secret_access_key(mut self, secret: impl Into<String>) -> Self {
        self.secret_access_key = Some(secret.into());
        self
    }

    pub fn build(self) -> S3Config {
        S3Config {
            region: self.region,
            credential_source: self.credential_source,
            access_key_id: self.access_key_id,
            secret_access_key: self.secret_access_key,
        }
    }
}
