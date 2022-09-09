use std::fs;
use std::path::Path;

use anyhow::Context;
use anyhow::Result;

use crate::cipd::common::GENERIC_HTTP_CLIENT;

pub async fn download<P: AsRef<Path>>(bucket: &str, hash: &str, destination_: P) -> Result<()> {
    let destination = destination_.as_ref();
    let url = format!(
        "https://commondatastorage.googleapis.com/{}/{}",
        bucket, hash
    );
    let request = GENERIC_HTTP_CLIENT
        .get(&url)
        .send()
        .await
        .with_context(|| format!("failed downloading url: {}", url))?
        .error_for_status()?;
    fs::write(destination, request.bytes().await?)
        .with_context(|| format!("writing file: {:?}", destination))?;
    Ok(())
}

pub async fn download_from_sha1_file<P: AsRef<Path>>(
    bucket: &str,
    sha1_file_location: P,
) -> Result<()> {
    let sha1_file = sha1_file_location.as_ref();
    let hash_ = fs::read_to_string(&sha1_file)
        .with_context(|| format!("reading sha1 file: {:?}", sha1_file))
        .unwrap();
    let hash = hash_.trim_end();
    assert_eq!(hash.len(), 40, "hash must be sha1 hex");
    assert!(
        hash.chars().all(|c| c.is_ascii_alphanumeric()),
        "hash must be sha1 hex"
    );
    let target_location = sha1_file.parent().unwrap().join(
        sha1_file
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .strip_suffix(".sha1")
            .unwrap(),
    );
    download(&bucket, &hash, &target_location)
        .await
        .with_context(|| {
            format!(
                "downloading {} from bucket {} to {:?}",
                hash, bucket, target_location
            )
        })?;
    Ok(())
}
