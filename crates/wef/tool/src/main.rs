mod commands;
mod internal;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::internal::{CefBuildsPlatform, DEFAULT_CEF_VERSION};

#[derive(Subcommand)]
enum Commands {
    /// Download CEF framework
    Init {
        /// Target path
        path: Option<PathBuf>,
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
    /// Compile a local package and all of its dependencies
    Build {
        /// Package to build (see `cargo help pkgid`)
        #[clap(long, short, value_name = "SPEC")]
        package: Option<String>,
        /// Build only the specified binary
        #[clap(long, value_name = "NAME")]
        bin: Option<String>,
        /// Build only the specified example
        #[clap(long, value_name = "NAME")]
        example: Option<String>,
        /// Build artifacts in release mode, with optimizations
        #[clap(long, short)]
        release: bool,
        /// Use the specified Wef version
        ///
        /// If not specified, use the latest version
        #[clap(long)]
        wef_version: Option<String>,
        /// Specify the source code path of the local Wef library instead of the
        /// published version
        #[clap(long)]
        wef_path: Option<PathBuf>,
        /// Specify the bundle type for the MacOS application
        bundle_type: Option<String>,
    },
    /// Run a binary or example of the local package
    Run {
        /// Package to build (see `cargo help pkgid`)
        #[clap(long, short, value_name = "SPEC")]
        package: Option<String>,
        /// Build only the specified binary
        #[clap(long, value_name = "NAME")]
        bin: Option<String>,
        /// Build only the specified example
        #[clap(long, value_name = "NAME")]
        example: Option<String>,
        /// Build artifacts in release mode, with optimizations
        #[clap(long, short)]
        release: bool,
        /// Use the specified Wef version
        ///
        /// If not specified, use the latest version
        #[clap(long)]
        wef_version: Option<String>,
        /// Specify the source code path of the local Wef library instead of the
        /// published version
        #[clap(long)]
        wef_path: Option<PathBuf>,
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// Add CEF framework to the application
    AddFramework {
        /// Target app path
        app_path: PathBuf,
        /// Build artifacts in release mode, with optimizations
        #[clap(long, short)]
        release: bool,
        /// Force adding the framework even if it already exists
        #[clap(long, short, default_value_t = false)]
        force: bool,
        /// Use the specified Wef version
        ///
        /// If not specified, use the latest version
        #[clap(long)]
        wef_version: Option<String>,
        /// Specify the source code path of the local Wef library instead of the
        /// published version
        #[clap(long)]
        wef_path: Option<PathBuf>,
    },
}

/// Wef CLI tool
#[derive(Parser)]
#[command(name = "cargo")]
#[command(bin_name = "cargo")]
#[clap(version, about)]
struct Cli {
    #[command(subcommand)]
    wef: WefCommands,
}

#[derive(Subcommand)]
enum WefCommands {
    Wef {
        #[command(subcommand)]
        commands: Commands,
    },
}

fn main() {
    let cli = Cli::parse();

    let res = match cli.wef {
        WefCommands::Wef {
            commands:
                Commands::Init {
                    path,
                    version,
                    platform,
                    force,
                },
        } => commands::init(path, version, platform, force),
        WefCommands::Wef {
            commands:
                Commands::Build {
                    package,
                    bin,
                    example,
                    release,
                    wef_version,
                    wef_path,
                    bundle_type,
                },
        } => commands::build(
            package,
            bin,
            example,
            release,
            wef_version.as_deref(),
            wef_path.as_deref(),
            bundle_type.as_deref(),
        )
        .map(|_| ()),
        WefCommands::Wef {
            commands:
                Commands::Run {
                    package,
                    bin,
                    example,
                    release,
                    wef_version,
                    wef_path,
                    args,
                },
        } => commands::run(
            package,
            bin,
            example,
            release,
            wef_version.as_deref(),
            wef_path.as_deref(),
            args,
        ),
        WefCommands::Wef {
            commands:
                Commands::AddFramework {
                    app_path,
                    release,
                    force,
                    wef_version,
                    wef_path,
                },
        } => commands::add_framework(app_path, release, force, wef_version, wef_path),
    };

    if let Err(err) = res {
        eprintln!("{:?}", err);
        std::process::exit(-1);
    }
}
