#[derive(Debug, Clone, Default)]
pub struct GcpDriveConfig {
    pub service_account_key_path: Option<String>,
}

impl GcpDriveConfig {
    pub fn builder() -> GcpDriveConfigBuilder {
        GcpDriveConfigBuilder::default()
    }
}

#[derive(Debug, Clone, Default)]
pub struct GcpDriveConfigBuilder {
    service_account_key_path: Option<String>,
}

impl GcpDriveConfigBuilder {
    pub fn service_account_key_path(mut self, path: impl Into<String>) -> Self {
        self.service_account_key_path = Some(path.into());
        self
    }

    pub fn build(self) -> GcpDriveConfig {
        GcpDriveConfig {
            service_account_key_path: self.service_account_key_path,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct GcpSheetsConfig {
    pub service_account_key_path: Option<String>,
}

impl GcpSheetsConfig {
    pub fn builder() -> GcpSheetsConfigBuilder {
        GcpSheetsConfigBuilder::default()
    }
}

#[derive(Debug, Clone, Default)]
pub struct GcpSheetsConfigBuilder {
    service_account_key_path: Option<String>,
}

impl GcpSheetsConfigBuilder {
    pub fn service_account_key_path(mut self, path: impl Into<String>) -> Self {
        self.service_account_key_path = Some(path.into());
        self
    }

    pub fn build(self) -> GcpSheetsConfig {
        GcpSheetsConfig {
            service_account_key_path: self.service_account_key_path,
        }
    }
}
