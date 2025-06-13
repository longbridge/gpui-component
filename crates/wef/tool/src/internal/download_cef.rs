use std::{
    fs::{self, File},
    io::{Read, Write},
    path::Path,
};

use anyhow::{Context, Result};
use reqwest::blocking::Client;
use tar::EntryType;

use crate::internal::CefBuildsPlatform;

pub(crate) fn download_cef(
    path: &Path,
    version: &str,
    platform: CefBuildsPlatform,
    force: bool,
    mut callback: impl DownloadCefCallback,
) -> Result<()> {
    if !force && path.exists() {
        return Ok(());
    }

    let url = platform
        .download_url(version)
        .ok_or_else(|| anyhow::anyhow!("unsupported platform: {:?}", platform))?;

    // Download with progress
    let client = Client::new();
    let response = client.get(&url).send().context("download CEF")?;

    let total_size = response.content_length().unwrap_or(0);

    let tmpdir_path = tempfile::tempdir().context("create temporary directory")?;
    let archive_path = tmpdir_path.path().join("cef.tar.bz2");

    callback.download_start(total_size);
    download_file(&url, &archive_path, &mut callback)?;
    callback.download_end();

    // Create the target directory if it doesn't exist
    fs::create_dir_all(path).context("create target directory")?;

    // Extract with progress
    callback.extract_start();
    extract_archive(
        &archive_path,
        path,
        &platform.root_dir_name(version).unwrap(),
        &mut callback,
    )
    .context("extract CEF archive")?;
    callback.extract_end();

    Ok(())
}

fn download_file(url: &str, path: &Path, callback: &mut impl DownloadCefCallback) -> Result<()> {
    let client = Client::new();

    let mut response = client
        .get(url)
        .send()
        .and_then(|resp| resp.error_for_status())
        .with_context(|| format!("download CEF from {}", url))?;

    let mut downloaded: u64 = 0;
    let mut buffer = [0; 8192];

    let mut file = File::create(path).with_context(|| format!("create file {}", path.display()))?;

    while let Ok(bytes_read) = response.read(&mut buffer) {
        if bytes_read == 0 {
            break;
        }
        file.write_all(&buffer[..bytes_read])
            .with_context(|| format!("write to file {}", path.display()))?;
        downloaded += bytes_read as u64;
        callback.download_progress(downloaded);
    }

    Ok(())
}

fn extract_archive(
    archive_path: &Path,
    target_dir: &Path,
    root_dir_name: &str,
    callback: &mut impl DownloadCefCallback,
) -> Result<()> {
    let tar_bz2 = File::open(archive_path)
        .with_context(|| format!("open archive file {}", archive_path.display()))?;

    let bz2 = bzip2::read::BzDecoder::new(tar_bz2);
    let mut archive = tar::Archive::new(bz2);

    let entries = archive.entries().context("read entries from archive")?;

    for res in entries {
        let mut entry = res.context("get entry from archive")?;

        if entry.header().entry_type() != EntryType::Regular {
            continue;
        }

        let entry_path = entry.path().unwrap().to_path_buf();
        let filepath = target_dir.join(entry_path.strip_prefix(root_dir_name).unwrap());
        let parent_path = filepath.parent().unwrap();
        std::fs::create_dir_all(parent_path)
            .with_context(|| format!("create directory for entry {}", parent_path.display()))?;

        entry.unpack(&filepath).with_context(|| {
            format!(
                "unpack entry {} to {}",
                entry_path.display(),
                filepath.display()
            )
        })?;
        callback.extract_file(&entry_path.display().to_string());
    }

    Ok(())
}

#[allow(unused_variables)]
pub(crate) trait DownloadCefCallback {
    fn download_start(&mut self, total: u64) {}

    fn download_progress(&mut self, downloaded: u64) {}

    fn download_end(&mut self) {}

    fn extract_start(&mut self) {}

    fn extract_file(&mut self, path: &str) {}

    fn extract_end(&mut self) {}
}
