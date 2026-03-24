use anyhow::{Context, Result};
use rcgen::{
    BasicConstraints, CertificateParams, DistinguishedName, DnType, IsCa, KeyPair,
    KeyUsagePurpose, SanType,
};
use rustls::pki_types::CertificateDer;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Manages CA certificate and dynamically generated server certificates.
///
/// We store the CA cert PEM and key PEM so we can reconstruct rcgen objects
/// on each signing operation (since rcgen 0.13 consumes CertificateParams).
pub struct CertManager {
    ca_cert_pem: String,
    ca_key_pem: String,
    cache_dir: PathBuf,
    /// In-memory cache: domain -> (cert_pem, key_pem)
    mem_cache: Arc<RwLock<HashMap<String, (String, String)>>>,
}

impl CertManager {
    /// Load or generate CA certificate from the given data directory
    pub fn new(data_dir: &Path) -> Result<Self> {
        let ca_dir = data_dir.join("certs").join("ca");
        let cache_dir = data_dir.join("certs").join("cache");
        fs::create_dir_all(&ca_dir)?;
        fs::create_dir_all(&cache_dir)?;

        let cert_path = ca_dir.join("cert.pem");
        let key_path = ca_dir.join("key.pem");

        let (ca_cert_pem, ca_key_pem) = if cert_path.exists() && key_path.exists() {
            let cert_pem = fs::read_to_string(&cert_path)
                .with_context(|| "Failed to read CA cert")?;
            let key_pem = fs::read_to_string(&key_path)
                .with_context(|| "Failed to read CA key")?;
            (cert_pem, key_pem)
        } else {
            let (cert_pem, key_pem) = generate_ca_cert()?;
            fs::write(&cert_path, &cert_pem)?;
            fs::write(&key_path, &key_pem)?;

            // Also write a copy for easy export
            let export_path = data_dir.join("certs").join("ca.crt");
            fs::write(&export_path, &cert_pem)?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&key_path, fs::Permissions::from_mode(0o600))?;
            }

            tracing::info!("Generated new CA certificate at {}", cert_path.display());
            (cert_pem, key_pem)
        };

        // Verify the key pair can be parsed
        let _ca_key_pair = KeyPair::from_pem(&ca_key_pem)
            .with_context(|| "Failed to parse CA key pair")?;

        Ok(Self {
            ca_cert_pem,
            ca_key_pem,
            cache_dir,
            mem_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Get the CA certificate PEM for export
    pub fn ca_cert_pem(&self) -> &str {
        &self.ca_cert_pem
    }

    /// Generate (or retrieve from cache) a server certificate for the given domain
    pub async fn get_or_create_cert(&self, domain: &str) -> Result<(String, String)> {
        // Check in-memory cache first
        {
            let cache = self.mem_cache.read().await;
            if let Some(cached) = cache.get(domain) {
                return Ok(cached.clone());
            }
        }

        // Check disk cache
        let cert_file = self.cache_dir.join(format!("{}.crt", domain));
        let key_file = self.cache_dir.join(format!("{}.key", domain));
        if cert_file.exists() && key_file.exists() {
            let cert_pem = fs::read_to_string(&cert_file)?;
            let key_pem = fs::read_to_string(&key_file)?;
            let mut cache = self.mem_cache.write().await;
            cache.insert(domain.to_string(), (cert_pem.clone(), key_pem.clone()));
            return Ok((cert_pem, key_pem));
        }

        // Generate new server certificate signed by our CA
        let (cert_pem, key_pem) = self.sign_server_cert(domain)?;

        // Write to disk cache
        fs::write(&cert_file, &cert_pem)?;
        fs::write(&key_file, &key_pem)?;

        // Update in-memory cache
        {
            let mut cache = self.mem_cache.write().await;
            cache.insert(domain.to_string(), (cert_pem.clone(), key_pem.clone()));
        }

        tracing::debug!("Generated server certificate for {}", domain);
        Ok((cert_pem, key_pem))
    }

    /// Sign a server certificate for the given domain using the CA.
    /// Reconstructs CA objects from PEM on each call (rcgen consumes params).
    fn sign_server_cert(&self, domain: &str) -> Result<(String, String)> {
        // Reconstruct CA key pair and certificate from stored PEM
        let ca_key_pair = KeyPair::from_pem(&self.ca_key_pem)
            .with_context(|| "Failed to parse CA key pair")?;

        let ca_cert_der = pem_to_der(&self.ca_cert_pem)?;
        let ca_params = CertificateParams::from_ca_cert_der(&ca_cert_der)
            .with_context(|| "Failed to parse CA cert params")?;
        // Re-self-sign to obtain a Certificate object usable as issuer
        let ca_cert = ca_params
            .self_signed(&ca_key_pair)
            .with_context(|| "Failed to reconstruct CA certificate")?;

        // Build server cert params
        let mut params = CertificateParams::new(vec![domain.to_string()])
            .with_context(|| format!("Failed to create cert params for {}", domain))?;

        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, domain);
        dn.push(DnType::OrganizationName, "FakeKey Proxy");
        params.distinguished_name = dn;

        params.subject_alt_names = vec![SanType::DnsName(domain.try_into()?)];

        let server_key_pair = KeyPair::generate()
            .with_context(|| "Failed to generate server key pair")?;

        let server_cert = params
            .signed_by(&server_key_pair, &ca_cert, &ca_key_pair)
            .with_context(|| format!("Failed to sign server cert for {}", domain))?;

        let cert_pem = server_cert.pem();
        let key_pem = server_key_pair.serialize_pem();

        Ok((cert_pem, key_pem))
    }

    /// Build a rustls ServerConfig for the given domain
    pub async fn make_server_config(&self, domain: &str) -> Result<Arc<rustls::ServerConfig>> {
        let (cert_pem, key_pem) = self.get_or_create_cert(domain).await?;

        let certs = rustls_pemfile::certs(&mut cert_pem.as_bytes())
            .collect::<Result<Vec<_>, _>>()
            .with_context(|| "Failed to parse server cert")?;

        let key = rustls_pemfile::private_key(&mut key_pem.as_bytes())
            .with_context(|| "Failed to parse server key")?
            .ok_or_else(|| anyhow::anyhow!("No private key found"))?;

        let config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .with_context(|| "Failed to build server TLS config")?;

        Ok(Arc::new(config))
    }
}

/// Generate a self-signed CA certificate
fn generate_ca_cert() -> Result<(String, String)> {
    let mut params = CertificateParams::new(Vec::<String>::new())
        .with_context(|| "Failed to create CA cert params")?;

    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, "FakeKey Root CA");
    dn.push(DnType::OrganizationName, "FakeKey");
    params.distinguished_name = dn;

    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    params.key_usages = vec![
        KeyUsagePurpose::KeyCertSign,
        KeyUsagePurpose::CrlSign,
    ];

    // Valid for 10 years
    params.not_before = rcgen::date_time_ymd(2024, 1, 1);
    params.not_after = rcgen::date_time_ymd(2034, 1, 1);

    let key_pair = KeyPair::generate()
        .with_context(|| "Failed to generate CA key pair")?;

    let cert = params.self_signed(&key_pair)
        .with_context(|| "Failed to self-sign CA certificate")?;

    Ok((cert.pem(), key_pair.serialize_pem()))
}

/// Convert PEM string to CertificateDer
fn pem_to_der(pem_str: &str) -> Result<CertificateDer<'static>> {
    let certs: Vec<_> = rustls_pemfile::certs(&mut pem_str.as_bytes())
        .collect::<Result<Vec<_>, _>>()
        .with_context(|| "Failed to parse PEM")?;
    certs
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("No certificate found in PEM"))
}
