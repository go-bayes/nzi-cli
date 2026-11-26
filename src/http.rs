//! shared http helpers for the remote data sources (exchange rates, weather)
//!
//! responses from these services are untrusted input: requests are https only,
//! time limited and bounded in size before any parsing happens

use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde::de::DeserializeOwned;

/// largest response body accepted from a data source
/// (real responses are a few kilobytes, so this leaves ample headroom)
pub const MAX_RESPONSE_BYTES: usize = 256 * 1024;

/// time allowed for a whole request, including reading the body
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// client settings shared by every data source
fn client_builder() -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        // refuse plain http, including a redirect from https to http
        .https_only(true)
}

/// build the http client for a data source, with an optional user agent
pub fn build_client(user_agent: Option<String>) -> reqwest::Client {
    let builder = match user_agent {
        Some(user_agent) => client_builder().user_agent(user_agent),
        None => client_builder(),
    };

    // a failed build means the tls backend could not start. falling back to
    // `Client::default()` would fail the same way, or else silently drop the
    // https-only and timeout settings, so stop here instead
    builder.build().expect("http client should initialise")
}

/// fetch a url and parse the body as json, rejecting oversized responses
pub async fn get_json<T: DeserializeOwned>(client: &reqwest::Client, url: &str) -> Result<T> {
    let response = client.get(url).send().await.context("request failed")?;
    let body = read_capped(response, MAX_RESPONSE_BYTES).await?;
    serde_json::from_slice(&body).context("response is not the expected json")
}

/// read a response body, stopping as soon as it exceeds `limit` bytes
async fn read_capped(mut response: reqwest::Response, limit: usize) -> Result<Vec<u8>> {
    // reject early when the server declares an oversized body
    if let Some(length) = response.content_length()
        && length > limit as u64
    {
        bail!("response too large ({} bytes, limit {})", length, limit);
    }

    // the declared length may be absent or wrong, so count what actually arrives
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .context("failed to read response body")?
    {
        if body.len() + chunk.len() > limit {
            bail!("response too large (limit {} bytes)", limit);
        }
        body.extend_from_slice(&chunk);
    }

    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    /// serve one canned http response on a loopback port and return its url
    async fn serve_once(response: Vec<u8>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("loopback listener should bind");
        let address = listener
            .local_addr()
            .expect("listener should have an address");

        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut request = [0u8; 1024];
                let _ = socket.read(&mut request).await;
                let _ = socket.write_all(&response).await;
                let _ = socket.shutdown().await;
            }
        });

        format!("http://{}/", address)
    }

    /// plain-http client for the loopback test server only
    fn test_client() -> reqwest::Client {
        reqwest::Client::builder()
            .no_proxy()
            .build()
            .expect("test client should build")
    }

    async fn fetch_capped(response: Vec<u8>, limit: usize) -> Result<Vec<u8>> {
        let url = serve_once(response).await;
        let response = test_client()
            .get(&url)
            .send()
            .await
            .expect("loopback request should succeed");
        read_capped(response, limit).await
    }

    fn chunked_response(chunks: &[&[u8]]) -> Vec<u8> {
        let mut response =
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n".to_vec();
        for chunk in chunks {
            response.extend_from_slice(format!("{:x}\r\n", chunk.len()).as_bytes());
            response.extend_from_slice(chunk);
            response.extend_from_slice(b"\r\n");
        }
        response.extend_from_slice(b"0\r\n\r\n");
        response
    }

    fn sized_response(body: &[u8]) -> Vec<u8> {
        let mut response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .into_bytes();
        response.extend_from_slice(body);
        response
    }

    #[tokio::test]
    async fn body_within_limit_is_returned() {
        let body = fetch_capped(sized_response(b"{\"ok\":true}"), 64)
            .await
            .expect("small body should be accepted");
        assert_eq!(body, b"{\"ok\":true}");
    }

    #[tokio::test]
    async fn body_at_limit_is_returned() {
        let body = fetch_capped(chunked_response(&[&[b'a'; 32], &[b'b'; 32]]), 64)
            .await
            .expect("body of exactly the limit should be accepted");
        assert_eq!(body.len(), 64);
    }

    #[tokio::test]
    async fn declared_oversized_body_is_rejected() {
        let error = fetch_capped(sized_response(&[b'a'; 65]), 64)
            .await
            .expect_err("declared oversized body should be rejected");
        assert!(error.to_string().contains("response too large"));
    }

    #[tokio::test]
    async fn chunked_oversized_body_is_rejected() {
        // no content-length header, so only the running count can catch this
        let error = fetch_capped(chunked_response(&[&[b'a'; 40], &[b'b'; 40]]), 64)
            .await
            .expect_err("chunked oversized body should be rejected");
        assert!(error.to_string().contains("response too large"));
    }

    #[tokio::test]
    async fn data_source_client_refuses_plain_http() {
        let url = serve_once(sized_response(b"{}")).await;
        let error = build_client(None)
            .get(&url)
            .send()
            .await
            .expect_err("plain http should be refused");
        // refused before any connection is attempted
        assert!(error.is_builder(), "unexpected error: {error:?}");
    }
}
