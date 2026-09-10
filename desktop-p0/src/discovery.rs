use std::path::{Path, PathBuf};

fn first_existing(paths: impl IntoIterator<Item = PathBuf>) -> Option<PathBuf> {
    paths
        .into_iter()
        .find(|path| path.is_absolute() && path.is_file())
}

pub fn github_cli() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|path| {
            std::env::split_paths(&path)
                .filter(|p| p.is_absolute())
                .map(|p| p.join("gh.exe"))
                .collect()
        })
        .unwrap_or_default();
    for (variable, suffix) in [
        ("ProgramFiles", "GitHub CLI/gh.exe"),
        ("LOCALAPPDATA", "Programs/GitHub CLI/gh.exe"),
        ("USERPROFILE", "scoop/shims/gh.exe"),
    ] {
        if let Some(root) = std::env::var_os(variable) {
            candidates.push(PathBuf::from(root).join(suffix));
        }
    }
    first_existing(candidates)
}

fn claude_report_in(root: &Path) -> Option<PathBuf> {
    first_existing([root.join("settings.agentmeter-statusline.observation.json")])
}

pub fn claude_report() -> Option<PathBuf> {
    let settings = claude_settings()?;
    first_existing([
        agentmeter_p0::claude_setup::report_path(&settings),
        claude_report_in(settings.parent()?).unwrap_or_default(),
    ])
}

pub fn claude_settings() -> Option<PathBuf> {
    let root = match std::env::var_os("CLAUDE_CONFIG_DIR") {
        Some(root) => PathBuf::from(root),
        None => PathBuf::from(std::env::var_os("USERPROFILE")?).join(".claude"),
    };
    root.is_absolute().then(|| root.join("settings.json"))
}

pub fn claude_cli() -> Option<PathBuf> {
    locate(
        "claude.exe",
        &[
            ("USERPROFILE", ".local/bin/claude.exe"),
            ("APPDATA", "npm/claude.cmd"),
        ],
    )
}

pub fn adb() -> Option<PathBuf> {
    locate(
        "adb.exe",
        &[
            ("LOCALAPPDATA", "Android/Sdk/platform-tools/adb.exe"),
            ("ANDROID_HOME", "platform-tools/adb.exe"),
            ("ANDROID_SDK_ROOT", "platform-tools/adb.exe"),
        ],
    )
}

fn locate(name: &str, known: &[(&str, &str)]) -> Option<PathBuf> {
    let mut candidates: Vec<_> = std::env::var_os("PATH")
        .map(|p| {
            std::env::split_paths(&p)
                .filter(|p| p.is_absolute())
                .map(|p| p.join(name))
                .collect()
        })
        .unwrap_or_default();
    for (variable, suffix) in known {
        if let Some(root) = std::env::var_os(variable) {
            candidates.push(PathBuf::from(root).join(suffix));
        }
    }
    first_existing(candidates)
}

pub fn claude_shell() -> Option<agentmeter_p0::claude_setup::DisplayShell> {
    use agentmeter_p0::claude_setup::DisplayShell;
    if let Some(path) = std::env::var_os("CLAUDE_CODE_GIT_BASH_PATH") {
        return first_existing([PathBuf::from(path)]).map(|executable| DisplayShell {
            executable,
            kind: "bash",
        });
    }
    // Do not mistake System32/bash.exe (WSL) for native Git Bash.
    let candidates = [
        ("ProgramFiles", "Git/bin/bash.exe"),
        ("LOCALAPPDATA", "Programs/Git/bin/bash.exe"),
    ]
    .into_iter()
    .filter_map(|(key, suffix)| std::env::var_os(key).map(|root| PathBuf::from(root).join(suffix)));
    if let Some(executable) = first_existing(candidates) {
        return Some(DisplayShell {
            executable,
            kind: "bash",
        });
    }
    locate(
        "powershell.exe",
        &[(
            "SystemRoot",
            "System32/WindowsPowerShell/v1.0/powershell.exe",
        )],
    )
    .map(|executable| DisplayShell {
        executable,
        kind: "powershell",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discovery_skips_missing_files_and_directories() {
        let root =
            std::env::temp_dir().join(format!("agentmeter-discovery-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join("settings.agentmeter-statusline.observation.json");
        std::fs::write(&file, b"{}").unwrap();
        assert_eq!(
            first_existing([
                PathBuf::from("relative.exe"),
                root.clone(),
                root.join("missing.exe"),
                file.clone()
            ]),
            Some(file.clone())
        );
        assert_eq!(claude_report_in(&root), Some(file.clone()));
        std::fs::remove_file(file).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
}
