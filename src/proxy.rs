use anyhow::{Context, Result};
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsAcceptor;
use tracing::{debug, error, info, warn};

use crate::audit::{AuditEventType, AuditLogger};
use crate::cert::CertManager;
use crate::key_handler;

/// Shared state for the proxy server
pub struct ProxyState {
    pub key_map: HashMap<String, String>,
    pub cert_manager: Arc<CertManager>,
    pub allowed_hosts: Vec<String>,
    pub audit_logger: Option<Arc<AuditLogger>>,
}

/// Start the proxy server on the given address
pub async fn start_proxy(addr: SocketAddr, state: Arc<ProxyState>) -> Result<()> {
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("Failed to bind to {}", addr))?;

    info!("Proxy server listening on {}", addr);

    loop {
        let (stream, client_addr) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                error!("Failed to accept connection: {}", e);
                continue;
            }
        };

        let state = state.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, client_addr, state).await {
                debug!("Connection from {} ended: {}", client_addr, e);
            }
        });
    }
}

/// Handle a single client connection
async fn handle_connection(
    stream: TcpStream,
    client_addr: SocketAddr,
    state: Arc<ProxyState>,
) -> Result<()> {
    let io = TokioIo::new(stream);
    let state_clone = state.clone();

    http1::Builder::new()
        .preserve_header_case(true)
        .title_case_headers(true)
        .serve_connection(
            io,
            service_fn(move |req| {
                let state = state_clone.clone();
                async move { handle_request(req, state).await }
            }),
        )
        .with_upgrades()
        .await
        .map_err(|e| anyhow::anyhow!("HTTP serve error from {}: {}", client_addr, e))
}

/// Route the request: CONNECT for HTTPS tunneling, otherwise plain HTTP proxy
async fn handle_request(
    req: Request<Incoming>,
    state: Arc<ProxyState>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    if req.method() == Method::CONNECT {
        handle_connect(req, state).await
    } else {
        handle_http(req, state).await
    }
}

/// Handle HTTP CONNECT method for HTTPS MITM proxy
async fn handle_connect(
    req: Request<Incoming>,
    state: Arc<ProxyState>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let host = req.uri().authority().map(|a| a.to_string()).unwrap_or_default();

    // Extract the domain (without port)
    let domain = host.split(':').next().unwrap_or(&host).to_string();

    debug!("CONNECT request to {}", host);

    // Check allowed hosts
    if !state.allowed_hosts.is_empty()
        && !state
            .allowed_hosts
            .iter()
            .any(|h| domain.contains(h) || h.contains(&domain))
    {
        warn!("Blocked CONNECT to non-allowed host: {}", domain);
        if let Some(logger) = &state.audit_logger {
            let _ = logger.log(
                AuditEventType::AuthFailure,
                format!("Blocked connection to non-allowed host: {}", domain),
                false,
            );
        }
        let resp = Response::builder()
            .status(StatusCode::FORBIDDEN)
            .body(Full::new(Bytes::from("Host not allowed")))
            .unwrap();
        return Ok(resp);
    }

    // Respond with 200 to establish the tunnel
    tokio::task::spawn(async move {
        match hyper::upgrade::on(req).await {
            Ok(upgraded) => {
                if let Err(e) = handle_tunnel(upgraded, &host, &domain, state).await {
                    error!("Tunnel error for {}: {}", host, e);
                }
            }
            Err(e) => {
                error!("Upgrade error for {}: {}", host, e);
            }
        }
    });

    Ok(Response::new(Full::new(Bytes::new())))
}

/// Handle the MITM tunnel: TLS accept from client, then proxy to upstream
async fn handle_tunnel(
    upgraded: hyper::upgrade::Upgraded,
    host: &str,
    domain: &str,
    state: Arc<ProxyState>,
) -> Result<()> {
    let server_config = state
        .cert_manager
        .make_server_config(domain)
        .await
        .with_context(|| format!("Failed to make server config for {}", domain))?;

    let tls_acceptor = TlsAcceptor::from(server_config);
    // Wrap Upgraded in TokioIo so it implements tokio AsyncRead/AsyncWrite
    let tokio_io = TokioIo::new(upgraded);
    let tls_stream = tls_acceptor
        .accept(tokio_io)
        .await
        .with_context(|| format!("TLS accept failed for {}", domain))?;

    // Wrap TLS stream in TokioIo again for hyper's Read/Write traits
    let io = TokioIo::new(tls_stream);
    let host = host.to_string();
    let domain = domain.to_string();
    let domain_for_err = domain.clone();
    let state = state.clone();

    http1::Builder::new()
        .preserve_header_case(true)
        .title_case_headers(true)
        .serve_connection(
            io,
            service_fn(move |req| {
                let state = state.clone();
                let host = host.clone();
                let domain = domain.clone();
                async move { handle_https_request(req, &host, &domain, state).await }
            }),
        )
        .await
        .map_err(|e| anyhow::anyhow!("HTTPS serve error for {}: {}", domain_for_err, e))?;

    Ok(())
}

/// Handle a decrypted HTTPS request: replace keys and forward to upstream
async fn handle_https_request(
    req: Request<Incoming>,
    host: &str,
    _domain: &str,
    state: Arc<ProxyState>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let uri_path = req.uri().path_and_query().map(|pq| pq.to_string()).unwrap_or_default();
    let upstream_uri = format!("https://{}{}", host, uri_path);

    debug!("HTTPS request: {} {}", req.method(), upstream_uri);

    match forward_request(req, &upstream_uri, &state.key_map, &state.audit_logger).await {
        Ok(resp) => Ok(resp),
        Err(e) => {
            error!("Forward error for {}: {}", upstream_uri, e);
            let resp = Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Full::new(Bytes::from(format!("Proxy error: {}", e))))
                .unwrap();
            Ok(resp)
        }
    }
}

/// Handle plain HTTP proxy request
async fn handle_http(
    req: Request<Incoming>,
    state: Arc<ProxyState>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let uri = req.uri().to_string();
    debug!("HTTP request: {} {}", req.method(), uri);

    match forward_request(req, &uri, &state.key_map, &state.audit_logger).await {
        Ok(resp) => Ok(resp),
        Err(e) => {
            error!("Forward error for {}: {}", uri, e);
            let resp = Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Full::new(Bytes::from(format!("Proxy error: {}", e))))
                .unwrap();
            Ok(resp)
        }
    }
}

/// Forward a request to upstream, replacing fake keys with real keys
async fn forward_request(
    req: Request<Incoming>,
    upstream_uri: &str,
    key_map: &HashMap<String, String>,
    audit_logger: &Option<Arc<AuditLogger>>,
) -> Result<Response<Full<Bytes>>> {
    let method = req.method().clone();
    let mut headers = req.headers().clone();

    let (final_uri, uri_replaced) = key_handler::replace_in_url(upstream_uri, key_map);
    if uri_replaced {
        info!("Replaced key in URL");
        if let Some(logger) = audit_logger {
            let _ = logger.log_key_replacement("URL");
        }
    }

    // Replace keys in headers
    let mut header_replacements = 0;
    let mut new_headers = hyper::HeaderMap::new();
    for (name, value) in headers.iter() {
        let value_str = value.to_str().unwrap_or_default();
        let (new_value, replaced) = key_handler::replace_in_header_value(value_str, key_map);
        if replaced {
            header_replacements += 1;
            info!("Replaced key in header: {}", name);
            if let Some(logger) = audit_logger {
                let _ = logger.log_key_replacement(&format!("Header: {}", name));
            }
        }
        if let Ok(v) = hyper::header::HeaderValue::from_str(&new_value) {
            new_headers.insert(name.clone(), v);
        } else {
            new_headers.insert(name.clone(), value.clone());
        }
    }
    headers = new_headers;

    // Remove proxy-related headers
    headers.remove("proxy-connection");
    headers.remove("proxy-authorization");

    // Read body
    let body_bytes = req
        .collect()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to read request body: {}", e))?
        .to_bytes();

    // Replace keys in body
    let (final_body, body_replaced) = key_handler::replace_in_body(&body_bytes, key_map);
    if body_replaced {
        info!("Replaced key in request body");
        if let Some(logger) = audit_logger {
            let _ = logger.log_key_replacement("Body");
        }
    }

    // Log request processing
    let key_replaced = uri_replaced || header_replacements > 0 || body_replaced;
    if let Some(logger) = audit_logger {
        let _ = logger.log_request(method.as_str(), upstream_uri, key_replaced);
    }

    // Build and send upstream request using hyper client
    let upstream_resp = send_upstream_request(&method, &final_uri, headers, final_body).await?;

    Ok(upstream_resp)
}

/// Send the request to the upstream server
async fn send_upstream_request(
    method: &Method,
    uri: &str,
    headers: hyper::HeaderMap,
    body: Vec<u8>,
) -> Result<Response<Full<Bytes>>> {
    // Parse the URI
    let parsed_uri: hyper::Uri = uri
        .parse()
        .with_context(|| format!("Invalid URI: {}", uri))?;

    let scheme = parsed_uri.scheme_str().unwrap_or("https");
    let host = parsed_uri
        .host()
        .ok_or_else(|| anyhow::anyhow!("No host in URI: {}", uri))?;
    let port = parsed_uri.port_u16().unwrap_or(if scheme == "https" { 443 } else { 80 });

    let addr = format!("{}:{}", host, port);

    // Connect to upstream
    let tcp_stream = TcpStream::connect(&addr)
        .await
        .with_context(|| format!("Failed to connect to {}", addr))?;

    if scheme == "https" {
        // TLS connection to upstream
        let mut root_store = rustls::RootCertStore::empty();
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

        let tls_config = rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        let connector = tokio_rustls::TlsConnector::from(Arc::new(tls_config));
        let server_name = rustls::pki_types::ServerName::try_from(host.to_string())
            .with_context(|| format!("Invalid server name: {}", host))?;

        let tls_stream = connector
            .connect(server_name, tcp_stream)
            .await
            .with_context(|| format!("TLS connect failed to {}", addr))?;

        send_via_connection(TokioIo::new(tls_stream), method, &parsed_uri, headers, body).await
    } else {
        send_via_connection(TokioIo::new(tcp_stream), method, &parsed_uri, headers, body).await
    }
}

/// Send request over an established connection and read the response
async fn send_via_connection<IO>(
    io: IO,
    method: &Method,
    uri: &hyper::Uri,
    headers: hyper::HeaderMap,
    body: Vec<u8>,
) -> Result<Response<Full<Bytes>>>
where
    IO: hyper::rt::Read + hyper::rt::Write + Unpin + Send + 'static,
{
    let (mut sender, conn) = hyper::client::conn::http1::Builder::new()
        .preserve_header_case(true)
        .title_case_headers(true)
        .handshake(io)
        .await
        .with_context(|| "HTTP handshake failed")?;

    // Spawn connection driver
    tokio::spawn(async move {
        if let Err(e) = conn.await {
            debug!("Upstream connection ended: {}", e);
        }
    });

    // Build path+query for the request
    let path_and_query = uri
        .path_and_query()
        .map_or_else(|| "/".to_string(), |pq| pq.to_string());

    let mut req_builder = Request::builder()
        .method(method.clone())
        .uri(path_and_query);

    // Copy headers
    for (name, value) in headers.iter() {
        // Skip hop-by-hop headers
        let name_str = name.as_str();
        if matches!(
            name_str,
            "transfer-encoding" | "connection" | "keep-alive" | "te" | "trailer" | "upgrade"
        ) {
            continue;
        }
        req_builder = req_builder.header(name, value);
    }

    // Ensure Host header is set
    if let Some(host) = uri.host() {
        let host_value = if let Some(port) = uri.port() {
            format!("{}:{}", host, port)
        } else {
            host.to_string()
        };
        req_builder = req_builder.header("host", host_value);
    }

    let upstream_req = req_builder
        .body(Full::new(Bytes::from(body)))
        .with_context(|| "Failed to build upstream request")?;

    let resp = sender
        .send_request(upstream_req)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to send upstream request: {}", e))?;

    // Read response body
    let (parts, incoming_body) = resp.into_parts();
    let resp_body = incoming_body
        .collect()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to read upstream response: {}", e))?
        .to_bytes();

    let response = Response::from_parts(parts, Full::new(resp_body));
    Ok(response)
}
