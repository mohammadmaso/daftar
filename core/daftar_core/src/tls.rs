//! One trust store for every platform (ADR-0015): the bundled Mozilla roots, used by rustls for AI
//! providers / MCP and written as a PEM file for libgit2's OpenSSL backend (Linux, Android), whose
//! compiled-in certificate paths do not exist on those systems.

use std::path::Path;
use std::sync::{Arc, OnceLock};

use base64::Engine;

fn provider() -> Arc<rustls::crypto::CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

/// rustls client config with bundled roots; shared by every HTTP client.
pub fn client_config() -> Arc<rustls::ClientConfig> {
    static CFG: OnceLock<Arc<rustls::ClientConfig>> = OnceLock::new();
    CFG.get_or_init(|| {
        let mut roots = rustls::RootCertStore::empty();
        roots.add_parsable_certificates(webpki_root_certs::TLS_SERVER_ROOT_CERTS.iter().cloned());
        Arc::new(
            rustls::ClientConfig::builder_with_provider(provider())
                .with_safe_default_protocol_versions()
                .expect("ring supports TLS 1.2/1.3")
                .with_root_certificates(roots)
                .with_no_client_auth(),
        )
    })
    .clone()
}

/// Proxy from the environment (`https_proxy`, `http_proxy`, `all_proxy`, `no_proxy`), never for
/// loopback hosts. Common for users behind censorship or corporate networks.
pub fn env_proxy_for(url: &reqwest::Url) -> Option<String> {
    let host = url
        .host_str()?
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_ascii_lowercase();
    if host == "localhost"
        || host
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
    {
        return None;
    }
    let var = |names: &[&str]| {
        names
            .iter()
            .find_map(|n| std::env::var(n).ok().filter(|v| !v.is_empty()))
    };
    if let Some(no) = var(&["no_proxy", "NO_PROXY"]) {
        let bypass = no
            .split(',')
            .map(|e| e.trim().trim_start_matches('.').to_ascii_lowercase())
            .any(|e| !e.is_empty() && (e == "*" || host == e || host.ends_with(&format!(".{e}"))));
        if bypass {
            return None;
        }
    }
    match url.scheme() {
        "https" => var(&["https_proxy", "HTTPS_PROXY", "all_proxy", "ALL_PROXY"]),
        _ => var(&["http_proxy", "HTTP_PROXY", "all_proxy", "ALL_PROXY"]),
    }
}

pub fn http_client(timeout: std::time::Duration) -> reqwest::Client {
    reqwest::Client::builder()
        .no_proxy()
        .proxy(reqwest::Proxy::custom(|url| {
            env_proxy_for(url).and_then(|p| reqwest::Url::parse(&p).ok())
        }))
        .use_preconfigured_tls((*client_config()).clone())
        .connect_timeout(std::time::Duration::from_secs(15))
        .timeout(timeout)
        .user_agent(format!("{}/{}", crate::APP_ID, crate::VERSION))
        .build()
        .expect("static client configuration is valid")
}

pub fn roots_pem() -> String {
    let b64 = base64::engine::general_purpose::STANDARD;
    let mut out = String::new();
    for cert in webpki_root_certs::TLS_SERVER_ROOT_CERTS {
        out.push_str("-----BEGIN CERTIFICATE-----\n");
        let enc = b64.encode(cert.as_ref());
        for chunk in enc.as_bytes().chunks(64) {
            out.push_str(std::str::from_utf8(chunk).expect("base64 is ascii"));
            out.push('\n');
        }
        out.push_str("-----END CERTIFICATE-----\n");
    }
    out
}

/// Points libgit2 at a CA bundle. Call once per process with a writable directory.
pub fn configure_git(state_dir: &Path) -> crate::Result<()> {
    static DONE: OnceLock<()> = OnceLock::new();
    if DONE.get().is_some() {
        return Ok(());
    }
    // libgit2 uses the OS TLS stack on Apple platforms (SecureTransport) and Windows (WinHTTP);
    // those use the system trust store and reject a CA file. Only the OpenSSL backend needs one.
    if cfg!(any(target_vendor = "apple", target_os = "windows")) {
        let _ = DONE.set(());
        let _ = state_dir;
        return Ok(());
    }
    // Prefer the OS bundle where it is a plain file (keeps enterprise CAs working on Linux).
    let system = [
        "/etc/ssl/certs/ca-certificates.crt",
        "/etc/pki/tls/certs/ca-bundle.crt",
    ]
    .into_iter()
    .map(Path::new)
    .find(|p| p.is_file());
    let file = match system {
        Some(p) => p.to_path_buf(),
        None => {
            let p = state_dir.join("ca-roots.pem");
            if !p.exists() {
                crate::fsutil::atomic_write(&p, roots_pem().as_bytes())?;
            }
            p
        }
    };
    // SAFETY: libgit2 global options; called once before any network operation.
    unsafe {
        git2::opts::set_ssl_cert_file(&file)?;
    }
    let _ = DONE.set(());
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn loopback_never_proxied() {
        let u = |s: &str| reqwest::Url::parse(s).unwrap();
        assert_eq!(super::env_proxy_for(&u("http://127.0.0.1:9/x")), None);
        assert_eq!(super::env_proxy_for(&u("http://localhost/x")), None);
        assert_eq!(super::env_proxy_for(&u("http://[::1]:8/x")), None);
    }

    #[test]
    fn roots_are_available() {
        let pem = super::roots_pem();
        assert!(pem.matches("BEGIN CERTIFICATE").count() > 100);
        let _ = super::client_config();
    }
}
