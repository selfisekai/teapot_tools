use anyhow::{bail, Result};
use once_cell::sync::Lazy;
use prost::{bytes::Bytes, DecodeError, Message};
use reqwest::{
    header::{HeaderMap, CONTENT_TYPE},
    Client,
};

use crate::host::{cipd_host_cpu, cipd_host_os};

static HTTP_HEADERS: Lazy<HeaderMap> = Lazy::new(|| {
    let mut default_headers = HeaderMap::new();
    default_headers.append(
        reqwest::header::CONTENT_TYPE,
        "application/prpc; encoding=binary".parse().unwrap(),
    );
    default_headers.append(
        reqwest::header::ACCEPT,
        "application/prpc; encoding=binary".parse().unwrap(),
    );
    default_headers
});

pub static CIPD_HTTP_CLIENT: Lazy<Client> = Lazy::new(|| {
    reqwest::ClientBuilder::new()
        .gzip(true)
        .no_brotli()
        .no_deflate()
        .default_headers(HTTP_HEADERS.clone())
        .user_agent(format!("teapot_tools/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .unwrap()
});

pub static GENERIC_HTTP_CLIENT: Lazy<Client> = Lazy::new(|| {
    reqwest::ClientBuilder::new()
        .gzip(true)
        .no_brotli()
        .no_deflate()
        .user_agent(format!("teapot_tools/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .unwrap()
});

pub async fn cipd_request<M: Message, R: Message, D: FnOnce(Bytes) -> Result<R, DecodeError>>(
    resource: &str,
    message: M,
    decoder: D,
) -> Result<R> {
    let res = CIPD_HTTP_CLIENT
        .post(format!(
            "https://chrome-infra-packages.appspot.com/prpc/{resource}"
        ))
        .body(message.encode_to_vec())
        .send()
        .await?;

    if !res.status().is_success() {
        if res
            .headers()
            .get(CONTENT_TYPE)
            .filter(|t| t.to_str().unwrap().starts_with("text/plain"))
            .is_some()
        {
            bail!(
                "cipd responded with http {}: {}",
                res.status(),
                res.text().await?
            );
        } else {
            bail!("cipd responded with http {}", res.status());
        }
    }

    Ok(decoder(res.bytes().await?).unwrap())
}

pub fn fill_host_variables(source: &str) -> String {
    return source
        .replace("${{platform}}", "${{os}}-${{arch}}")
        .replace("${{os}}", &cipd_host_os())
        .replace("${{arch}}", &cipd_host_cpu());
}
