use std::path::PathBuf;

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};

use crate::internal::{CefBuildsPlatform, DownloadCefCallback};

#[derive(Debug, Default)]
struct CmdDownloadCallback {
    download_progress: Option<ProgressBar>,
    extract_progress: Option<ProgressBar>,
}

pub(crate) fn init(
    path: Option<PathBuf>,
    version: String,
    platform: CefBuildsPlatform,
    force: bool,
) -> Result<()> {
    let path = path.unwrap_or_else(|| dirs::home_dir().expect("get home directory").join(".cef"));
    crate::internal::download_cef(
        &path,
        &version,
        platform,
        force,
        CmdDownloadCallback::default(),
    )?;
    println!("Set environment variable CEF_ROOT={}", path.display());
    Ok(())
}

fn create_download_progress_bar() -> ProgressBar {
    ProgressBar::new_spinner().with_style(
        ProgressStyle::default_spinner()
            .progress_chars("#>-" )
            .template("Downloading CEF {spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}) {eta}").unwrap()
    )
}

fn create_extract_progress_bar() -> ProgressBar {
    ProgressBar::new_spinner().with_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    )
}

impl DownloadCefCallback for CmdDownloadCallback {
    fn download_start(&mut self, total_size: u64) {
        self.download_progress = Some(create_download_progress_bar());
        if let Some(pb) = &self.download_progress {
            pb.set_length(total_size);
        }
    }

    fn download_progress(&mut self, downloaded: u64) {
        if let Some(pb) = &self.download_progress {
            pb.set_position(downloaded);
        }
    }

    fn download_end(&mut self) {
        if let Some(pb) = self.download_progress.take() {
            pb.finish_and_clear();
            println!("Download complete");
        }
    }

    fn extract_start(&mut self) {
        self.extract_progress = Some(create_extract_progress_bar());
    }

    fn extract_file(&mut self, path: &str) {
        if let Some(pb) = &self.extract_progress {
            pb.set_message(format!("Extracting {}", path));
        }
    }

    fn extract_end(&mut self) {
        if let Some(pb) = self.extract_progress.take() {
            pb.finish_and_clear();
            println!("Extraction complete");
        }
    }
}
