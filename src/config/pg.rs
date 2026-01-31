#[derive(Debug, Clone, Default)]
pub struct PgConfig {
    pub username: Option<String>,
    pub password: Option<String>,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub database: Option<String>,
    pub max_connections: Option<u32>,
    pub credentials_json: Option<String>,
}

impl PgConfig {
    pub fn builder() -> PgConfigBuilder {
        PgConfigBuilder::default()
    }
}

#[derive(Debug, Clone, Default)]
pub struct PgConfigBuilder {
    username: Option<String>,
    password: Option<String>,
    host: Option<String>,
    port: Option<u16>,
    database: Option<String>,
    max_connections: Option<u32>,
    credentials_json: Option<String>,
}

impl PgConfigBuilder {
    pub fn username(mut self, username: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self
    }

    pub fn password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }

    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = Some(host.into());
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    pub fn database(mut self, database: impl Into<String>) -> Self {
        self.database = Some(database.into());
        self
    }

    pub fn max_connections(mut self, max_connections: u32) -> Self {
        self.max_connections = Some(max_connections);
        self
    }

    pub fn credentials_json(mut self, credentials: impl Into<String>) -> Self {
        self.credentials_json = Some(credentials.into());
        self
    }

    pub fn build(self) -> PgConfig {
        PgConfig {
            username: self.username,
            password: self.password,
            host: self.host,
            port: self.port,
            database: self.database,
            max_connections: self.max_connections,
            credentials_json: self.credentials_json,
        }
    }
}
