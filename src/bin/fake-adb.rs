//! Test-only ADB process double. Behavior is supplied only to isolated child tests.
use std::{fs::OpenOptions, io::Write, process::ExitCode, time::Duration};

fn append_log(arguments: &[String]) {
    if let Some(path) = std::env::var_os("AGENTMETER_FAKE_ADB_LOG") {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .unwrap();
        writeln!(file, "{}", arguments.join(" ")).unwrap();
    }
}

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    append_log(&arguments);
    let mode = std::env::var("AGENTMETER_FAKE_ADB_MODE").unwrap_or_default();
    if mode == "timeout" {
        std::thread::sleep(Duration::from_secs(30));
        return ExitCode::SUCCESS;
    }
    if arguments == ["devices", "-l"] {
        print!(
            "{}",
            std::env::var("AGENTMETER_FAKE_ADB_DEVICES").unwrap_or_default()
        );
        return ExitCode::SUCCESS;
    }
    if arguments.get(2..4) == Some(&["reverse".into(), "--no-rebind".into()]) {
        if mode == "conflict" {
            eprintln!("cannot rebind existing socket");
            return ExitCode::from(1);
        }
        if let Some(path) = std::env::var_os("AGENTMETER_FAKE_ADB_MAPPING") {
            std::fs::write(
                path,
                format!("{} {} {}\n", arguments[1], arguments[4], arguments[5]),
            )
            .unwrap();
        }
        return ExitCode::SUCCESS;
    }
    if arguments.get(2..4) == Some(&["reverse".into(), "--list".into()]) {
        let mut count = 0;
        if let Some(path) = std::env::var_os("AGENTMETER_FAKE_ADB_COUNT") {
            count = std::fs::read_to_string(&path)
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(0);
            std::fs::write(path, (count + 1).to_string()).unwrap();
        }
        let key = if count == 0 {
            "AGENTMETER_FAKE_ADB_LIST"
        } else {
            "AGENTMETER_FAKE_ADB_LIST_AFTER"
        };
        let output = std::env::var(key)
            .ok()
            .or_else(|| std::env::var("AGENTMETER_FAKE_ADB_LIST").ok())
            .or_else(|| {
                std::env::var_os("AGENTMETER_FAKE_ADB_MAPPING")
                    .and_then(|path| std::fs::read_to_string(path).ok())
            })
            .unwrap_or_default();
        print!("{output}");
        return ExitCode::SUCCESS;
    }
    if arguments.get(2..4) == Some(&["reverse".into(), "--remove".into()]) {
        if let Some(path) = std::env::var_os("AGENTMETER_FAKE_ADB_MAPPING") {
            let _ = std::fs::remove_file(path);
        }
        return ExitCode::SUCCESS;
    }
    if arguments.get(2..6) == Some(&["shell".into(), "am".into(), "start".into(), "-W".into()]) {
        if mode == "launch-failed" {
            eprintln!("activity manager rejected request");
            return ExitCode::from(1);
        }
        return ExitCode::SUCCESS;
    }
    eprintln!("unexpected fake adb arguments");
    ExitCode::from(2)
}
