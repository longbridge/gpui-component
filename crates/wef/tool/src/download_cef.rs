use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use anyhow::Result;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::blocking::Client;
use tar::EntryType;

use crate::{cef_platform::CefBuildsPlatform, utils::print_error};

#[derive(Debug)]
pub(crate) struct DownloadCefSettings {
    pub(crate) path: PathBuf,
    pub(crate) version: String,
    pub(crate) platform: CefBuildsPlatform,
    pub(crate) force: bool,
}

fn create_download_progress_bar() -> ProgressBar {
    let pb = ProgressBar::new(0);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}) {eta}")
            .unwrap()
            .progress_chars("#>-"),
    );
    pb
}

fn create_extract_progress_bar() -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message("Extracting files...");
    pb
}

fn download_file(url: &str, pb: &ProgressBar, path: &Path) -> Result<()> {
    let client = Client::new();

    println!("Downloading CEF from {}...", url);

    let mut response = client.get(url).send().inspect_err(|err| {
        print_error(format_args!("failed to download CEF: {}", err));
    })?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "failed to download CEF: HTTP {}",
            response.status()
        ));
    }

    let content_length = response
        .content_length()
        .ok_or_else(|| anyhow::anyhow!("failed to get content length"))?;

    pb.set_length(content_length);

    let mut downloaded: u64 = 0;
    let mut buffer = [0; 8192];

    let mut file = File::create(path).inspect_err(|err| {
        print_error(format_args!(
            "failed to create file {}: {}",
            path.display(),
            err
        ));
    })?;

    while let Ok(bytes_read) = response.read(&mut buffer) {
        if bytes_read == 0 {
            break;
        }
        file.write_all(&buffer[..bytes_read]).inspect_err(|err| {
            print_error(format_args!("failed to write to file: {}", err));
        })?;
        downloaded += bytes_read as u64;
        pb.set_position(downloaded);
    }

    pb.finish_with_message("Download completed");
    Ok(())
}

fn extract_archive(
    archive_path: &Path,
    target_dir: &Path,
    root_dir_name: &str,
    pb: &ProgressBar,
) -> Result<()> {
    std::fs::create_dir_all(target_dir).inspect_err(|err| {
        print_error(format_args!(
            "failed to create target directory {}: {}",
            target_dir.display(),
            err
        ));
    })?;

    let tar_bz2 = File::open(archive_path).inspect_err(|err| {
        print_error(format_args!(
            "failed to open archive {}: {}",
            archive_path.display(),
            err
        ));
    })?;

    let bz2 = bzip2::read::BzDecoder::new(tar_bz2);
    let mut archive = tar::Archive::new(bz2);

    let entries = archive.entries().inspect_err(|err| {
        print_error(format_args!(
            "failed to read entries from archive {}: {}",
            archive_path.display(),
            err
        ));
    })?;

    for res in entries {
        let mut entry = res.inspect_err(|err| {
            print_error(format_args!(
                "failed to read entry from archive {}: {}",
                archive_path.display(),
                err
            ));
        })?;

        if entry.header().entry_type() != EntryType::Regular {
            continue;
        }

        let entry_path = entry.path().unwrap();
        let filepath = target_dir.join(entry_path.strip_prefix(root_dir_name).unwrap());
        std::fs::create_dir_all(filepath.parent().unwrap()).inspect_err(|err| {
            print_error(format_args!(
                "failed to create directory for {}: {}",
                filepath.display(),
                err
            ));
        })?;

        entry.unpack(target_dir.join(filepath)).inspect_err(|err| {
            print_error(format_args!(
                "failed to extract file to {}: {}",
                target_dir.display(),
                err
            ));
        })?;
        pb.set_message(entry.path().unwrap().display().to_string());
    }

    pb.finish_with_message("Extraction completed");
    Ok(())
}

pub(crate) fn download_cef(settings: &DownloadCefSettings) -> Result<()> {
    if !settings.force && settings.path.exists() {
        println!(
            "CEF already exists at {}. Use {} to re-download.",
            "--force".bright_white(),
            settings.path.display()
        );
    }

    let url = settings
        .platform
        .download_url(&settings.version)
        .ok_or_else(|| anyhow::anyhow!("unsupported platform: {:?}", settings.platform))?;

    println!("Downloading CEF from {}...", url);

    // Download with progress
    let client = Client::new();
    let response = client.get(&url).send().inspect_err(|err| {
        print_error(format_args!("failed to download CEF: {}", err));
    })?;

    let total_size = response.content_length().unwrap_or(0);
    let pb = create_download_progress_bar();

    pb.set_length(total_size);

    let tmpdir_path = tempfile::tempdir().inspect_err(|err| {
        print_error(format_args!(
            "failed to create temporary directory: {}",
            err
        ));
    })?;
    let archive_path = tmpdir_path.path().join("cef.tar.bz2");
    download_file(&url, &pb, &archive_path)?;

    pb.finish_with_message("Download completed");

    println!("Extracting CEF to {}...", settings.path.display());

    // Create the target directory if it doesn't exist
    fs::create_dir_all(&settings.path).inspect_err(|err| {
        print_error(format_args!(
            "failed to create directory {}: {}",
            settings.path.display(),
            err
        ));
    })?;

    // Extract with progress
    let pb = create_extract_progress_bar();
    extract_archive(
        &archive_path,
        &settings.path,
        &settings.platform.root_dir_name(&settings.version).unwrap(),
        &pb,
    )?;
    pb.finish_with_message("Extraction completed");

    println!("{}", "Successfully downloaded and extracted CEF!".green());
    println!();

    println!(
        "Set the environment variable CEF_ROOT = {}",
        settings.path.display()
    );
    Ok(())
}
