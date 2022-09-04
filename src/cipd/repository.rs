use anyhow::Result;
use prost::Message;

use crate::types::cipd::GetInstanceUrlRequest;
use crate::types::cipd::InstanceDigest;
use crate::types::cipd::InstanceUrl;
use crate::types::cipd::PackageInstance;
use crate::types::cipd::ResolveVersionRequest;

use super::common::cipd_request;
use super::common::fill_host_variables;

pub async fn resolve_instance(package: &str, tag: &str) -> Result<PackageInstance> {
    cipd_request(
        "cipd.Repository/ResolveVersion",
        ResolveVersionRequest {
            package: fill_host_variables(package),
            tag: tag.to_string(),
        },
        PackageInstance::decode,
    )
    .await
}

pub async fn get_instance_url(package: &str, digest: &InstanceDigest) -> Result<String> {
    cipd_request(
        "cipd.Repository/GetInstanceURL",
        GetInstanceUrlRequest {
            package: package.to_string(),
            digest: Some(digest.clone()),
        },
        InstanceUrl::decode,
    )
    .await
    .map(|i| i.url)
}

#[cfg(test)]
mod tests {
    use crate::types::cipd::InstanceDigest;

    use super::{get_instance_url, resolve_instance};

    #[tokio::test]
    async fn test_resolve_instance() {
        let package = "dart/third_party/flutter/devtools";
        let solve = resolve_instance(
            package,
            "git_revision:40aae5e5ea2118e2b6dee8a8a20f166f7cec4270",
        )
        .await
        .unwrap();

        assert_eq!(solve.package, package);

        let digest = solve.digest.unwrap();
        assert_eq!(digest.algorithm, 2); // sha256, why are enums broken?
        assert_eq!(
            digest.hex_digest,
            "c64ba943bcce4b54d9ea87479b95b308bfb0ed699c87aa55fb0bfe15b94e7b66"
        );

        assert_eq!(solve.publisher, "user:kenzieschmoll@google.com");
    }

    #[tokio::test]
    async fn test_get_url() {
        let package = "dart/third_party/flutter/devtools";
        let solve = get_instance_url(
            package,
            &InstanceDigest {
                algorithm: 2,
                hex_digest: "c64ba943bcce4b54d9ea87479b95b308bfb0ed699c87aa55fb0bfe15b94e7b66"
                    .to_string(),
            },
        )
        .await
        .unwrap();

        assert!(solve.starts_with("https://storage.googleapis.com/chrome-infra-packages/store/SHA256/c64ba943bcce4b54d9ea87479b95b308bfb0ed699c87aa55fb0bfe15b94e7b66"));
    }
}
