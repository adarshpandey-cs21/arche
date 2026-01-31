#[derive(Debug, Clone, Default)]
pub struct AwsConfig {
    pub region: Option<String>,
}

impl AwsConfig {
    pub fn builder() -> AwsConfigBuilder {
        AwsConfigBuilder::default()
    }
}

#[derive(Debug, Clone, Default)]
pub struct AwsConfigBuilder {
    region: Option<String>,
}

impl AwsConfigBuilder {
    pub fn region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }

    pub fn build(self) -> AwsConfig {
        AwsConfig {
            region: self.region,
        }
    }
}
