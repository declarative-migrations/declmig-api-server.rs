#![forbid(unsafe_code)]

use std::{collections::BTreeMap, net::SocketAddr};

#[derive(Clone, Debug)]
pub struct ApiConfig {
    pub bind: String,
    pub tcp_bind: Option<String>,
    pub nats_url: Option<String>,
}

impl ApiConfig {
    pub fn from_env() -> Self {
        Self::from_map(&std::env::vars().collect())
    }

    pub fn from_map(environment: &BTreeMap<String, String>) -> Self {
        Self {
            bind: environment
                .get("DECLMIG_API_BIND")
                .cloned()
                .unwrap_or_else(|| "127.0.0.1:8080".into()),
            tcp_bind: environment.get("DECLMIG_API_TCP_BIND").cloned(),
            nats_url: environment.get("DECLMIG_NATS_URL").cloned(),
        }
    }
}

impl ApiConfig {
    /// Reject invalid transport endpoints before the API can start.
    /// Error strings deliberately never echo credentials or connection strings.
    pub fn validate(&self) -> Result<(), &'static str> {
        validate_bind(&self.bind, "DECLMIG_API_BIND")?;
        if let Some(tcp_bind) = &self.tcp_bind {
            validate_bind(tcp_bind, "DECLMIG_API_TCP_BIND")?;
        }
        if let Some(nats_url) = &self.nats_url {
            let authority = nats_url
                .strip_prefix("nats://")
                .or_else(|| nats_url.strip_prefix("tls://"))
                .ok_or("DECLMIG_NATS_URL requires nats:// or tls://")?;
            if authority.is_empty()
                || authority.chars().any(char::is_whitespace)
                || authority.starts_with('/')
                || authority.contains('#')
            {
                return Err("DECLMIG_NATS_URL must contain a nonempty authority");
            }
        }
        Ok(())
    }
}

fn validate_bind(value: &str, name: &'static str) -> Result<(), &'static str> {
    let address: SocketAddr = value.parse().map_err(|_| name)?;
    if address.port() == 0 {
        return Err(name);
    }
    Ok(())
}

#[cfg(test)]
mod endpoint_tests {
    use super::*;

    #[test]
    fn valid_ipv4_ipv6_and_nats_endpoints() {
        let env = BTreeMap::from([
            ("DECLMIG_API_BIND".into(), "[::1]:8080".into()),
            ("DECLMIG_API_TCP_BIND".into(), "127.0.0.1:9090".into()),
            ("DECLMIG_NATS_URL".into(), "tls://nats.internal:4222".into()),
        ]);
        assert!(ApiConfig::from_map(&env).validate().is_ok());
        assert!(ApiConfig::from_map(&BTreeMap::new()).validate().is_ok());
    }

    #[test]
    fn explicit_invalid_endpoints_fail_closed() {
        let cases = [
            ("DECLMIG_API_BIND", "localhost:8080"),
            ("DECLMIG_API_BIND", "127.0.0.1:0"),
            ("DECLMIG_API_BIND", "0.0.0.0:not-a-port"),
            ("DECLMIG_API_TCP_BIND", ""),
            ("DECLMIG_API_TCP_BIND", "127.0.0.1:0"),
            ("DECLMIG_NATS_URL", "https://nats.example"),
            ("DECLMIG_NATS_URL", "nats://"),
            ("DECLMIG_NATS_URL", "nats://bad host"),
            ("DECLMIG_NATS_URL", "nats://host#fragment"),
        ];
        for (key, value) in cases {
            let env = BTreeMap::from([(key.to_owned(), value.to_owned())]);
            let result = ApiConfig::from_map(&env).validate();
            assert!(result.is_err(), "{key} must reject invalid input");
            assert!(!result.unwrap_err().contains(value), "error must not echo input");
        }
    }
}
