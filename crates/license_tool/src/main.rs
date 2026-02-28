use anyhow::{Result, anyhow, bail};
use one_core::license::PlanTier;
use std::env;

mod offline_license;

use offline_license::{
    build_offline_license_payload, generate_keypair_base64, generate_offline_license_document,
    signing_key_from_base64, write_offline_license_to_path,
};

struct IssueArgs {
    user_id: String,
    plan: PlanTier,
    expires_at: Option<i64>,
    device_id: Option<String>,
    secret_key_base64: String,
    output_path: String,
}

enum Command {
    GenerateKeypair,
    Issue(IssueArgs),
}

fn main() {
    if let Err(error) = run() {
        eprintln!("错误: {}", error);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let arguments: Vec<String> = env::args().collect();
    let command = parse_args(&arguments)?;

    let Some(command) = command else {
        print_usage();
        return Ok(());
    };

    match command {
        Command::GenerateKeypair => {
            let (secret_key_base64, public_key_base64) = generate_keypair_base64();
            println!("SECRET_KEY_BASE64={}", secret_key_base64);
            println!("PUBLIC_KEY_BASE64={}", public_key_base64);
        }
        Command::Issue(args) => {
            let signing_key = signing_key_from_base64(&args.secret_key_base64)?;
            let payload = build_offline_license_payload(
                args.user_id,
                args.plan,
                args.expires_at,
                args.device_id,
            );
            let document = generate_offline_license_document(&payload, &signing_key)?;
            write_offline_license_to_path(&document, &args.output_path)?;
            println!("OK: {}", args.output_path);
        }
    }

    Ok(())
}

fn parse_args(arguments: &[String]) -> Result<Option<Command>> {
    if arguments.len() < 2 {
        return Ok(None);
    }

    match arguments[1].as_str() {
        "-h" | "--help" | "help" => Ok(None),
        "generate-keypair" | "gen-keys" => {
            if arguments.len() > 2 {
                bail!("generate-keypair 不支持参数");
            }
            Ok(Some(Command::GenerateKeypair))
        }
        "issue" => {
            let issue_args = parse_issue_args(&arguments[2..])?;
            Ok(Some(Command::Issue(issue_args)))
        }
        other => Err(anyhow!("未知命令: {}", other)),
    }
}

fn parse_issue_args(arguments: &[String]) -> Result<IssueArgs> {
    let mut user_id: Option<String> = None;
    let mut plan: Option<PlanTier> = None;
    let mut expires_at: Option<i64> = None;
    let mut device_id: Option<String> = None;
    let mut secret_key_base64: Option<String> = None;
    let mut output_path: Option<String> = None;

    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--user-id" => {
                let value = require_next_value(arguments, index, "--user-id")?;
                user_id = Some(value);
                index += 2;
            }
            "--plan" => {
                let value = require_next_value(arguments, index, "--plan")?;
                plan = Some(parse_plan(&value)?);
                index += 2;
            }
            "--expires-at" => {
                let value = require_next_value(arguments, index, "--expires-at")?;
                let parsed = value
                    .parse::<i64>()
                    .map_err(|_| anyhow!("expires-at 不是有效时间戳"))?;
                expires_at = Some(parsed);
                index += 2;
            }
            "--device-id" => {
                let value = require_next_value(arguments, index, "--device-id")?;
                device_id = Some(value);
                index += 2;
            }
            "--secret-key" => {
                let value = require_next_value(arguments, index, "--secret-key")?;
                secret_key_base64 = Some(value);
                index += 2;
            }
            "--output" => {
                let value = require_next_value(arguments, index, "--output")?;
                output_path = Some(value);
                index += 2;
            }
            other => {
                bail!("未知参数: {}", other);
            }
        }
    }

    let user_id = user_id.ok_or_else(|| anyhow!("缺少 --user-id"))?;
    let plan = plan.ok_or_else(|| anyhow!("缺少 --plan"))?;
    let secret_key_base64 = secret_key_base64.ok_or_else(|| anyhow!("缺少 --secret-key"))?;
    let output_path = output_path.ok_or_else(|| anyhow!("缺少 --output"))?;

    Ok(IssueArgs {
        user_id,
        plan,
        expires_at,
        device_id,
        secret_key_base64,
        output_path,
    })
}

fn require_next_value(arguments: &[String], index: usize, name: &str) -> Result<String> {
    arguments
        .get(index + 1)
        .cloned()
        .ok_or_else(|| anyhow!("缺少 {}", name))
}

fn parse_plan(value: &str) -> Result<PlanTier> {
    match value.to_lowercase().as_str() {
        "free" => Ok(PlanTier::Free),
        "pro" => Ok(PlanTier::Pro),
        _ => Err(anyhow!("未知 plan: {}", value)),
    }
}

fn print_usage() {
    println!("Usage:");
    println!("  license_tool generate-keypair");
    println!(
        "  license_tool issue --user-id <id> --plan <free|pro> --secret-key <base64> --output <path> [--expires-at <unix>] [--device-id <id>]"
    );
}
