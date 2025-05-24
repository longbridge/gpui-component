mod add_cef_framework;
mod add_helper;
mod cef_platform;
mod download_cef;
mod utils;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::cef_platform::{CefBuildsPlatform, DEFAULT_CEF_VERSION};

#[derive(Subcommand)]
enum Commands {
    DownloadCef {
        /// CEF version
        #[clap(long, default_value = DEFAULT_CEF_VERSION)]
        version: String,
        /// Platform
        #[clap(long, default_value = "auto")]
        platform: CefBuildsPlatform,
        /// Target path
        path: PathBuf,
    },
    /// Add helper processes to the app
    AddHelper {
        /// Target app path
        app_path: PathBuf,
        /// CEF root path
        #[clap(long, env = "CEF_ROOT")]
        cef_root: Option<PathBuf>,
        /// Use the specified Wef version
        ///
        /// If not specified, use the latest version
        #[clap(long)]
        wef_version: Option<String>,
        /// Specify the source code path of the local Wef library instead of the
        /// published version
        #[clap(long)]
        wef_path: Option<PathBuf>,
        /// Build artifacts in release mode, with optimizations
        #[clap(long, short)]
        release: bool,
    },
    /// Add CEF framework to the app
    AddFramework {
        /// Target app path
        app_path: PathBuf,
        /// CEF root path
        #[clap(long, env = "CEF_ROOT")]
        cef_root: Option<PathBuf>,
        /// Build artifacts in release mode, with optimizations
        #[clap(long, short)]
        release: bool,
    },
}

/// Wef CLI tool
#[derive(Parser)]
#[clap(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::DownloadCef {
            version,
            platform,
            path,
        } => {
            _ = download_cef::download_cef(&version, platform, &path);
        }
        Commands::AddHelper {
            app_path,
            cef_root,
            wef_version,
            wef_path,
            release,
        } => {
            let settings = add_helper::AddHelperSettings {
                cef_root,
                app_path,
                wef_version,
                wef_path,
                release,
            };
            _ = add_helper::add_helper(&settings);
        }
        Commands::AddFramework {
            app_path,
            cef_root,
            release,
        } => {
            let settings = add_cef_framework::AddCefFrameworkSettings {
                cef_root,
                app_path,
                release,
            };
            _ = add_cef_framework::add_cef_framework(&settings);
        }
    }
}
