#[derive(Debug, Clone, Default)]
pub struct CloudFrontConfig {
    pub region: Option<String>,
    pub distribution_id: Option<String>,
}

impl CloudFrontConfig {
    pub fn builder() -> CloudFrontConfigBuilder {
        CloudFrontConfigBuilder::default()
    }
}

#[derive(Debug, Clone, Default)]
pub struct CloudFrontConfigBuilder {
    region: Option<String>,
    distribution_id: Option<String>,
}

impl CloudFrontConfigBuilder {
    pub fn region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }

    pub fn distribution_id(mut self, distribution_id: impl Into<String>) -> Self {
        self.distribution_id = Some(distribution_id.into());
        self
    }

    pub fn build(self) -> CloudFrontConfig {
        CloudFrontConfig {
            region: self.region,
            distribution_id: self.distribution_id,
        }
    }
}
