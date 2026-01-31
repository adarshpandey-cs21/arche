#[derive(Debug, Clone, Default)]
pub struct RedisConfig {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub max_connections: Option<u32>,
    pub password: Option<String>,
}

impl RedisConfig {
    pub fn builder() -> RedisConfigBuilder {
        RedisConfigBuilder::default()
    }
}

#[derive(Debug, Clone, Default)]
pub struct RedisConfigBuilder {
    host: Option<String>,
    port: Option<u16>,
    max_connections: Option<u32>,
    password: Option<String>,
}

impl RedisConfigBuilder {
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = Some(host.into());
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    pub fn max_connections(mut self, max_connections: u32) -> Self {
        self.max_connections = Some(max_connections);
        self
    }

    pub fn password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }

    pub fn build(self) -> RedisConfig {
        RedisConfig {
            host: self.host,
            port: self.port,
            max_connections: self.max_connections,
            password: self.password,
        }
    }
}
