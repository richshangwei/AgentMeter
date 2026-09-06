mod codex;
mod model;

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use codex::{collect_fixture, collect_live};

enum CollectionInput {
    Fixture(PathBuf),
    Live {
        codex_bin: PathBuf,
        timeout: Duration,
    },
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let input = match parse_input(&args) {
        Ok(input) => input,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };

    let result = match input {
        CollectionInput::Fixture(path) => collect_fixture(&path),
        CollectionInput::Live { codex_bin, timeout } => collect_live(&codex_bin, timeout),
    };

    match result {
        Ok(report) => {
            serde_json::to_writer_pretty(std::io::stdout(), &report)
                .expect("serialize collection report");
            println!();
            ExitCode::SUCCESS
        }
        Err(report) => {
            serde_json::to_writer_pretty(std::io::stdout(), &report)
                .expect("serialize failure report");
            println!();
            ExitCode::from(1)
        }
    }
}

fn parse_input(args: &[String]) -> Result<CollectionInput, String> {
    if args.len() < 2 || args[0] != "codex" || args[1] != "collect" {
        return Err(usage());
    }
    let mut fixture = None;
    let mut codex_bin = PathBuf::from("codex");
    let mut timeout = Duration::from_millis(5_000);
    let mut index = 2;
    while index < args.len() {
        let value = args.get(index + 1).ok_or_else(usage)?;
        match args[index].as_str() {
            "--fixture" => fixture = Some(PathBuf::from(value)),
            "--codex-bin" => codex_bin = PathBuf::from(value),
            "--timeout-ms" => {
                let millis = value
                    .parse::<u64>()
                    .map_err(|_| "--timeout-ms must be a positive integer".to_owned())?;
                if millis == 0 {
                    return Err("--timeout-ms must be greater than zero".to_owned());
                }
                timeout = Duration::from_millis(millis);
            }
            _ => return Err(usage()),
        }
        index += 2;
    }

    Ok(match fixture {
        Some(path) => CollectionInput::Fixture(path),
        None => CollectionInput::Live { codex_bin, timeout },
    })
}

fn usage() -> String {
    "usage: agentmeter-p0 codex collect [--fixture <transcript.jsonl>] [--codex-bin <path>] [--timeout-ms <milliseconds>]".to_owned()
}
