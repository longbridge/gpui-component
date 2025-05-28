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
    /// Download CEF framework
    DownloadCef {
        /// Target path
        path: PathBuf,
        /// CEF version
        #[clap(long, default_value = DEFAULT_CEF_VERSION)]
        version: String,
        /// Platform
        #[clap(long, default_value = "auto")]
        platform: CefBuildsPlatform,
        /// Force download even if the file already exists
        #[clap(long, short, default_value_t = false)]
        force: bool,
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
        /// Force adding the helper processes even if they already exist
        #[clap(long, short, default_value_t = false)]
        force: bool,
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
        /// Force adding the framework even if it already exists
        #[clap(long, short, default_value_t = false)]
        force: bool,
    },
}

/// Wef CLI tool
#[derive(Parser)]
#[clap(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

fn run_command(f: impl FnOnce() -> anyhow::Result<()>) {
    match f() {
        Ok(_) => std::process::exit(0),
        Err(_) => std::process::exit(-1),
    }
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::DownloadCef {
            path,
            version,
            platform,
            force,
        } => {
            let settings = download_cef::DownloadCefSettings {
                path,
                version,
                platform,
                force,
            };
            run_command(|| download_cef::download_cef(&settings))
        }
        Commands::AddHelper {
            app_path,
            cef_root,
            wef_version,
            wef_path,
            release,
            force,
        } => {
            let settings = add_helper::AddHelperSettings {
                cef_root,
                app_path,
                wef_version,
                wef_path,
                release,
                force,
            };
            run_command(|| add_helper::add_helper(&settings))
        }
        Commands::AddFramework {
            app_path,
            cef_root,
            release,
            force,
        } => {
            let settings = add_cef_framework::AddCefFrameworkSettings {
                cef_root,
                app_path,
                release,
                force,
            };
            run_command(|| add_cef_framework::add_cef_framework(&settings))
        }
    }
}
