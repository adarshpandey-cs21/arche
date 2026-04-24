use google_drive3::{
    hyper_rustls::{self, HttpsConnector},
    hyper_util::{
        client::legacy::{Client, connect::HttpConnector},
        rt::TokioExecutor,
    },
    yup_oauth2::{ServiceAccountAuthenticator, ServiceAccountKey, authenticator::Authenticator},
};
use hyper_proxy2::{Intercept, Proxy, ProxyConnector};
use std::sync::OnceLock;

use crate::error::AppError;

pub(crate) type ProxiedConnector = ProxyConnector<HttpsConnector<HttpConnector>>;
pub(crate) type ProxiedAuthenticator = Authenticator<ProxiedConnector>;

struct ProxySettings {
    url: Option<String>,
    no_proxy: Vec<String>,
}

static SETTINGS: OnceLock<ProxySettings> = OnceLock::new();

fn settings() -> &'static ProxySettings {
    SETTINGS.get_or_init(|| {
        let url = std::env::var("HTTPS_PROXY")
            .ok()
            .or_else(|| std::env::var("https_proxy").ok())
            .or_else(|| std::env::var("HTTP_PROXY").ok())
            .or_else(|| std::env::var("http_proxy").ok())
            .or_else(|| std::env::var("ALL_PROXY").ok())
            .or_else(|| std::env::var("all_proxy").ok())
            .filter(|s| !s.is_empty());

        let no_proxy = std::env::var("NO_PROXY")
            .or_else(|_| std::env::var("no_proxy"))
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();

        ProxySettings { url, no_proxy }
    })
}

fn host_bypasses_proxy(host: Option<&str>, no_proxy: &[String]) -> bool {
    let Some(host) = host else {
        return false;
    };
    let host = host.to_lowercase();
    no_proxy.iter().any(|p| {
        let pattern = p.trim_start_matches('.');
        host == pattern || host.ends_with(&format!(".{pattern}"))
    })
}

fn redact_proxy_url(uri: &http::Uri) -> String {
    let scheme = uri.scheme_str().unwrap_or("?");
    let host = uri.host().unwrap_or("?");
    match uri.port_u16() {
        Some(port) => format!("{scheme}://{host}:{port}"),
        None => format!("{scheme}://{host}"),
    }
}

pub(crate) fn build_proxy_aware_connector(service: &str) -> Result<ProxiedConnector, AppError> {
    let https = hyper_rustls::HttpsConnectorBuilder::new()
        .with_native_roots()
        .map_err(|e| {
            AppError::internal_error(
                format!("Failed to build HTTPS connector for {service}: {e}"),
                None,
            )
        })?
        .https_or_http()
        .enable_http1()
        .enable_http2()
        .build();

    let s = settings();
    let Some(raw) = s.url.as_deref() else {
        return Ok(ProxyConnector::unsecured(https));
    };

    let uri = raw.parse::<http::Uri>().map_err(|e| {
        AppError::internal_error(format!("Invalid proxy URL for {service}: {e}"), None)
    })?;

    if uri.scheme_str() == Some("https") {
        return Err(AppError::internal_error(
            format!(
                "HTTPS proxy scheme not supported for {service} (proxy={}); rebuild hyper-proxy2 with a TLS feature",
                redact_proxy_url(&uri)
            ),
            None,
        ));
    }

    let no_proxy = s.no_proxy.clone();
    let intercept = Intercept::from(
        move |_scheme: Option<&str>, host: Option<&str>, _port: Option<u16>| {
            !host_bypasses_proxy(host, &no_proxy)
        },
    );

    Ok(ProxyConnector::from_proxy_unsecured(
        https,
        Proxy::new(intercept, uri),
    ))
}

pub(crate) fn build_auth_hyper_client(
    service: &str,
) -> Result<Client<ProxiedConnector, String>, AppError> {
    let connector = build_proxy_aware_connector(service)?;
    Ok(Client::builder(TokioExecutor::new())
        .pool_max_idle_per_host(0)
        .build::<_, String>(connector))
}

pub(crate) async fn build_sa_authenticator(
    service: &str,
    sa_key: ServiceAccountKey,
) -> Result<ProxiedAuthenticator, AppError> {
    let hyper_client = build_auth_hyper_client(service)?;
    ServiceAccountAuthenticator::builder(sa_key)
        .hyper_client(hyper_client)
        .build()
        .await
        .map_err(|e| {
            AppError::internal_error(
                format!("Failed to build {service} authenticator: {e}"),
                None,
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_proxy_suffix_and_exact_match() {
        let patterns = vec![".googleapis.com".to_string(), "localhost".to_string()];
        assert!(host_bypasses_proxy(
            Some("oauth2.googleapis.com"),
            &patterns
        ));
        assert!(host_bypasses_proxy(Some("googleapis.com"), &patterns));
        assert!(host_bypasses_proxy(Some("localhost"), &patterns));
        assert!(!host_bypasses_proxy(Some("example.com"), &patterns));
        assert!(!host_bypasses_proxy(Some("notgoogleapis.com"), &patterns));
        assert!(!host_bypasses_proxy(None, &patterns));
    }

    #[test]
    fn no_proxy_case_insensitive() {
        let patterns = vec!["googleapis.com".to_string()];
        assert!(host_bypasses_proxy(
            Some("OAuth2.GoogleAPIs.com"),
            &patterns
        ));
    }

    #[test]
    fn redact_proxy_url_strips_userinfo() {
        let uri: http::Uri = "http://user:pass@proxy.example:8080/path".parse().unwrap();
        assert_eq!(redact_proxy_url(&uri), "http://proxy.example:8080");
    }
}
