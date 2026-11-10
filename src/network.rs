//! Network access state shared by the CLI and daemon.
//!
//! A host home opens a time-limited network window. The daemon owns the
//! window (`network.json`) and the host identity (`network-identity.json`:
//! bearer token, TLS certificate and key). A client
//! home instead connects to a remote daemon (`remote.json`), in which case it
//! never starts or stops a local daemon. A home cannot be both.

use crate::{models::*, store};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use uuid::Uuid;

pub const DEFAULT_BIND: &str = "0.0.0.0:7676";
/// A window never ends more than this far in the future.
pub const MAX_WINDOW_SECONDS: u64 = 24 * 60 * 60;
const WINDOW_FILE: &str = "network.json";
const IDENTITY_FILE: &str = "network-identity.json";
const REMOTE_FILE: &str = "remote.json";

/// An open network window, persisted so a restarted daemon restores it
/// until (never beyond) its expiry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Window {
    pub bind: String,
    /// Unix seconds.
    pub expires_at: i64,
}

/// Client-side remote daemon configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteConfig {
    /// Always `https://host:port`.
    pub url: String,
    pub token: String,
    /// Pinned SHA-256 fingerprint of the host certificate.
    pub fingerprint: String,
}

fn config_error(message: impl Into<String>, hint: &str) -> Failure {
    Failure::new("NETWORK_CONFIG", message, hint)
}

fn read<T: DeserializeOwned>(name: &str) -> Result<Option<T>> {
    let path = store::home().join(name);
    match std::fs::read(&path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map(Some).map_err(|e| {
            config_error(
                format!("Could not parse {}: {e}", path.display()),
                "Remove the file, then run tala network open or tala network connect again.",
            )
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

fn write_bytes(name: &str, bytes: &[u8]) -> Result<()> {
    store::private_dir(&store::home())?;
    store::private_write(&store::home().join(name), bytes)?;
    Ok(())
}

fn write<T: Serialize>(name: &str, value: &T) -> Result<()> {
    write_bytes(
        name,
        &serde_json::to_vec_pretty(value).expect("network state serializes"),
    )
}

/// Removes a state file; returns whether it existed.
fn remove(name: &str) -> Result<bool> {
    match std::fs::remove_file(store::home().join(name)) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.into()),
    }
}

pub fn window() -> Result<Option<Window>> {
    read(WINDOW_FILE)
}
pub fn save_window(window: &Window) -> Result<()> {
    write(WINDOW_FILE, window)
}
pub fn remove_window() -> Result<bool> {
    remove(WINDOW_FILE)
}

/// The host's network credentials: bearer token plus TLS certificate and
/// key, stored together so they are always replaced as a set.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HostIdentity {
    pub token: String,
    /// Hex-encoded certificate DER.
    cert: String,
    /// Hex-encoded PKCS#8 private key DER.
    key: String,
}

impl HostIdentity {
    pub fn cert_der(&self) -> Result<Vec<u8>> {
        from_hex(&self.cert).ok_or_else(corrupt_identity)
    }
    pub fn key_der(&self) -> Result<Vec<u8>> {
        from_hex(&self.key).ok_or_else(corrupt_identity)
    }
    pub fn fingerprint(&self) -> Result<String> {
        Ok(fingerprint(&self.cert_der()?))
    }
    /// Join code for one reachable address, e.g. `192.168.1.20:7676`.
    pub fn join_code(&self, address: &str) -> Result<String> {
        Ok(format!(
            "tala://{}@{address}#{}",
            self.token,
            self.fingerprint()?
        ))
    }
}

fn corrupt_identity() -> Failure {
    config_error(
        format!("{IDENTITY_FILE} is corrupt"),
        "Run tala network rotate-token to create new credentials; clients must connect again.",
    )
}

/// SHA-256 of a certificate's DER, as lowercase hex.
pub fn fingerprint(cert_der: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    to_hex(&Sha256::digest(cert_der))
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn from_hex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(text.get(i..i + 2)?, 16).ok())
        .collect()
}

fn generate_identity() -> Result<HostIdentity> {
    let failed = |e: rcgen::Error| {
        Failure::new(
            "NETWORK_CONFIG",
            format!("Could not generate a TLS certificate: {e}"),
            "Report this failure.",
        )
    };
    let key = rcgen::KeyPair::generate().map_err(failed)?;
    let mut params = rcgen::CertificateParams::new(vec!["tala".to_owned()]).map_err(failed)?;
    params
        .distinguished_name
        .push(rcgen::DnType::CommonName, "Tala daemon");
    let cert = params.self_signed(&key).map_err(failed)?;
    Ok(HostIdentity {
        token: new_token(),
        cert: to_hex(cert.der()),
        key: to_hex(&key.serialize_der()),
    })
}

pub fn host_identity() -> Result<Option<HostIdentity>> {
    read(IDENTITY_FILE)
}
/// Returns the existing identity, creating one on first use.
pub fn host_identity_or_create() -> Result<HostIdentity> {
    match host_identity()? {
        Some(identity) => Ok(identity),
        None => rotate_host_identity(),
    }
}
/// Replaces the token, certificate and key together.
pub fn rotate_host_identity() -> Result<HostIdentity> {
    let identity = generate_identity()?;
    write(IDENTITY_FILE, &identity)?;
    Ok(identity)
}

/// A parsed join code: `tala://<token>@<host:port>#<fingerprint>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinCode {
    pub token: String,
    pub address: String,
    pub fingerprint: String,
}

pub fn parse_join_code(code: &str) -> Result<JoinCode> {
    let invalid = || {
        config_error(
            "Join code is malformed",
            "Paste the whole join code from tala network open (or tala network status --show-token) on the host.",
        )
    };
    let rest = code.trim().strip_prefix("tala://").ok_or_else(invalid)?;
    let (rest, fingerprint) = rest.rsplit_once('#').ok_or_else(invalid)?;
    let (token, address) = rest.split_once('@').ok_or_else(invalid)?;
    validate_token(token).map_err(|_| invalid())?;
    if fingerprint.len() != 64 || !fingerprint.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(invalid());
    }
    normalise_url(&format!("https://{address}")).map_err(|_| invalid())?;
    Ok(JoinCode {
        token: token.to_owned(),
        address: address.to_owned(),
        fingerprint: fingerprint.to_ascii_lowercase(),
    })
}

pub fn remote() -> Result<Option<RemoteConfig>> {
    read(REMOTE_FILE)
}
pub fn save_remote(config: &RemoteConfig) -> Result<()> {
    write(REMOTE_FILE, config)
}
pub fn remove_remote() -> Result<bool> {
    remove(REMOTE_FILE)
}

/// Parses a duration such as `45s`, `90m`, `2h` or `1d`. A unit is required
/// so that a bare number is never misread.
pub fn parse_duration(text: &str) -> std::result::Result<u64, String> {
    let text = text.trim();
    let split = text
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(text.len());
    let (digits, unit) = text.split_at(split);
    let value: u64 = digits.parse().map_err(|_| {
        format!("invalid duration {text:?}; use a number with s, m, h or d, such as 90m")
    })?;
    let scale = match unit {
        "s" => 1,
        "m" => 60,
        "h" => 3600,
        "d" => 86_400,
        _ => {
            return Err(format!(
                "invalid duration {text:?}; use a number with s, m, h or d, such as 90m"
            ))
        }
    };
    match value.checked_mul(scale) {
        Some(0) | None => Err(format!("duration {text:?} must be positive and at most 1d")),
        Some(seconds) => Ok(seconds),
    }
}

/// 244 random bits from the OS RNG (two v4 UUIDs), hex-encoded.
pub fn new_token() -> String {
    format!("tnt_{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

/// Constant-time equality, so response timing does not reveal a token prefix.
pub fn token_matches(expected: &str, presented: &str) -> bool {
    let (a, b) = (expected.as_bytes(), presented.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

pub fn validate_token(token: &str) -> Result<()> {
    if token.len() < 32 || !token.chars().all(|c| c.is_ascii_graphic()) {
        return Err(config_error(
            "Access token is missing or malformed",
            "Paste the token shown by tala network open on the host.",
        ));
    }
    Ok(())
}

/// Validates a bind address of the form `IP:PORT` (port 0 picks a free port).
pub fn validate_bind(bind: &str) -> Result<std::net::SocketAddr> {
    bind.parse().map_err(|_| {
        config_error(
            format!("Invalid bind address {bind:?}"),
            "Use IP:PORT, such as 0.0.0.0:7676 for all interfaces or 192.168.1.20:7676 for one.",
        )
    })
}

/// Normalises a remote URL to `https://host:port` with no trailing path.
/// The network listener speaks only TLS, so `http://` is rejected.
pub fn normalise_url(url: &str) -> Result<String> {
    let url = url.trim();
    let (scheme, rest) = url.split_once("://").unwrap_or(("https", url));
    let rest = rest.strip_suffix('/').unwrap_or(rest);
    let valid_host = !rest.is_empty()
        && !rest.contains('/')
        && !rest.contains('@')
        && !rest.contains('#')
        && !rest.chars().any(char::is_whitespace);
    if scheme != "https" || !valid_host {
        return Err(config_error(
            format!("Invalid remote URL {url:?}"),
            "Use https://HOST:PORT, for example https://192.168.1.20:7676.",
        ));
    }
    Ok(format!("https://{rest}"))
}

/// Non-loopback IPv4 addresses of this machine, best effort, for join hints.
/// Private LAN ranges come first, since they are the usual target.
pub fn local_ipv4_addresses() -> Vec<String> {
    let mut addresses: Vec<std::net::Ipv4Addr> = Vec::new();
    #[cfg(unix)]
    unsafe {
        let mut list: *mut libc::ifaddrs = std::ptr::null_mut();
        if libc::getifaddrs(&mut list) == 0 {
            let mut cursor = list;
            while !cursor.is_null() {
                let entry = &*cursor;
                if !entry.ifa_addr.is_null() && (*entry.ifa_addr).sa_family as i32 == libc::AF_INET
                {
                    let addr = &*(entry.ifa_addr as *const libc::sockaddr_in);
                    let ip = std::net::Ipv4Addr::from(u32::from_be(addr.sin_addr.s_addr));
                    if !ip.is_loopback() && !ip.is_link_local() {
                        addresses.push(ip);
                    }
                }
                cursor = entry.ifa_next;
            }
            libc::freeifaddrs(list);
        }
    }
    addresses.sort_by_key(|ip| (!ip.is_private(), *ip));
    addresses.dedup();
    addresses.into_iter().map(|ip| ip.to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_long_unique_and_compared_exactly() {
        let a = new_token();
        let b = new_token();
        assert_ne!(a, b);
        assert_eq!(a.len(), 68);
        validate_token(&a).unwrap();
        assert!(token_matches(&a, &a.clone()));
        assert!(!token_matches(&a, &b));
        assert!(!token_matches(&a, &a[..a.len() - 1]));
        assert!(validate_token("short").is_err());
    }

    #[test]
    fn urls_are_normalised_and_validated() {
        assert_eq!(
            normalise_url("192.168.1.20:7676").unwrap(),
            "https://192.168.1.20:7676"
        );
        assert_eq!(
            normalise_url("https://mac.local:7676/").unwrap(),
            "https://mac.local:7676"
        );
        for bad in [
            "",
            "ftp://x:1",
            "http://x:1",
            "https://x:1/path",
            "https://u@x:1",
            "https://",
        ] {
            assert!(normalise_url(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn identities_round_trip_through_join_codes() {
        let identity = generate_identity().unwrap();
        let fp = identity.fingerprint().unwrap();
        assert_eq!(fp.len(), 64);
        assert_eq!(fingerprint(&identity.cert_der().unwrap()), fp);
        assert!(!identity.key_der().unwrap().is_empty());
        let code = identity.join_code("192.168.1.20:7676").unwrap();
        let parsed = parse_join_code(&format!("  {code}\n")).unwrap();
        assert_eq!(parsed.token, identity.token);
        assert_eq!(parsed.address, "192.168.1.20:7676");
        assert_eq!(parsed.fingerprint, fp);
        assert_ne!(generate_identity().unwrap().fingerprint().unwrap(), fp);
        for bad in [
            "".to_owned(),
            code.replace("tala://", "http://"),
            code.replace('#', "%"),
            format!("{}0", code),
            code.replace(&identity.token, "short"),
        ] {
            assert!(parse_join_code(&bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn durations_need_a_unit() {
        assert_eq!(parse_duration("45s"), Ok(45));
        assert_eq!(parse_duration("90m"), Ok(5400));
        assert_eq!(parse_duration("2h"), Ok(7200));
        assert_eq!(parse_duration("1d"), Ok(86_400));
        for bad in ["", "90", "0m", "h", "1.5h", "2w", "-1h"] {
            assert!(parse_duration(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn bind_addresses_must_be_ip_and_port() {
        assert!(validate_bind("0.0.0.0:7676").is_ok());
        assert!(validate_bind("[::]:7676").is_ok());
        assert!(validate_bind("localhost:7676").is_err());
        assert!(validate_bind("0.0.0.0").is_err());
    }
}
