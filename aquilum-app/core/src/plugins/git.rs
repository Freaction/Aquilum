use regex::Regex;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::{Arc, Mutex, OnceLock};

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "code", content = "details", rename_all = "snake_case")]
pub enum GitError {
    NotARepository { message: String },
    GitNotFound { message: String },
    Conflict { message: String, files: Vec<String> },
    Auth { message: String },
    Failed { message: String },
    Task { message: String },
}

impl From<crate::TaskFailed> for GitError {
    fn from(error: crate::TaskFailed) -> Self {
        Self::Task {
            message: mask_url_credentials(&error.to_string()),
        }
    }
}

impl From<std::io::Error> for GitError {
    fn from(error: std::io::Error) -> Self {
        Self::Failed {
            message: mask_url_credentials(&error.to_string()),
        }
    }
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitStatus {
    pub branch: String,
    pub ahead: usize,
    pub behind: usize,
    pub changed: usize,
    pub conflicted: Vec<String>,
}

use super::dates::LocalDateTime;

/// `{{date}}` в шаблоне сообщения — момент коммита в формате `commitDateFormat` (токены moment).
pub fn commit_message(template: &str, date_format: &str, now: &LocalDateTime) -> String {
    template.replace("{{date}}", &super::moment::format(date_format, now, "en"))
}

fn mask_url_credentials(message: &str) -> String {
    static URL: OnceLock<Regex> = OnceLock::new();
    URL.get_or_init(|| {
        Regex::new(r#"([A-Za-z][A-Za-z0-9+.-]*://)[^\s/"'<>]+@"#).expect("шаблон URL")
    })
    .replace_all(message, "${1}***@")
    .into_owned()
}

fn git(root: &Path, args: &[&str]) -> Result<Output, GitError> {
    let mut command = Command::new("git");
    command
        .args(args)
        .current_dir(root)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("LC_ALL", "C")
        .stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    command.output().map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            GitError::GitNotFound {
                message: "git executable not found".to_owned(),
            }
        } else {
            error.into()
        }
    })
}

fn error_from_stderr(stderr: &str) -> GitError {
    let message = mask_url_credentials(stderr.trim());
    let lower = stderr.to_ascii_lowercase();
    if lower.contains("not a git repository") {
        GitError::NotARepository { message }
    } else if [
        "authentication failed", "could not read username", "could not read password",
        "permission denied", "publickey", "terminal prompts disabled",
        "http 401", "http 403", "returned error: 401", "returned error: 403",
    ].iter().any(|text| lower.contains(text)) {
        GitError::Auth { message }
    } else {
        GitError::Failed { message }
    }
}

fn checked(root: &Path, args: &[&str]) -> Result<Output, GitError> {
    let output = git(root, args)?;
    if output.status.success() {
        Ok(output)
    } else {
        Err(error_from_stderr(&String::from_utf8_lossy(&output.stderr)))
    }
}

fn parse_status(text: &str) -> GitStatus {
    let mut status = GitStatus::default();
    let nul = text.contains('\0');
    let mut records = text.split(if nul { '\0' } else { '\n' });
    while let Some(record) = records.next() {
        if let Some(branch) = record.strip_prefix("# branch.head ") {
            status.branch = branch.to_owned();
        } else if let Some(counts) = record.strip_prefix("# branch.ab ") {
            let mut fields = counts.split_whitespace();
            status.ahead = fields.next().and_then(|n| n.strip_prefix('+'))
                .and_then(|n| n.parse().ok()).unwrap_or(0);
            status.behind = fields.next().and_then(|n| n.strip_prefix('-'))
                .and_then(|n| n.parse().ok()).unwrap_or(0);
        } else if record.starts_with("1 ") || record.starts_with("? ") {
            status.changed += 1;
        } else if record.starts_with("2 ") {
            status.changed += 1;
            if nul {
                records.next();
            }
        } else if record.starts_with("u ") {
            status.changed += 1;
            if let Some(path) = record.splitn(11, ' ').nth(10) {
                status.conflicted.push(path.to_owned());
            }
        }
    }
    status
}

fn status_unlocked(root: &Path) -> Result<GitStatus, GitError> {
    let output = checked(root, &[
        "status", "--porcelain=v2", "--branch", "--untracked-files=all", "-z",
        "--", ".", ":(exclude).aquilum/history",
    ])?;
    Ok(parse_status(&String::from_utf8_lossy(&output.stdout)))
}

fn with_lock<T>(
    root: &Path,
    operation: impl FnOnce(&Path) -> Result<T, GitError>,
) -> Result<T, GitError> {
    static LOCKS: OnceLock<Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>> = OnceLock::new();
    let root = root.canonicalize()?;
    let lock = LOCKS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock().map_err(|_| GitError::Failed { message: "git lock poisoned".to_owned() })?
        .entry(root.clone()).or_default().clone();
    let _guard = lock.lock()
        .map_err(|_| GitError::Failed { message: "git lock poisoned".to_owned() })?;
    operation(&root)
}

fn ensure_no_conflicts(status: &GitStatus) -> Result<(), GitError> {
    if status.conflicted.is_empty() {
        return Ok(());
    }
    Err(GitError::Conflict {
        message: "Unresolved git conflicts".to_owned(),
        files: status.conflicted.clone(),
    })
}

fn pull_unlocked(root: &Path, method: &str) -> Result<(), GitError> {
    let flag = match method {
        "merge" => "--no-rebase",
        "rebase" => "--rebase",
        _ => return Err(GitError::Failed {
            message: "Unknown git sync method".to_owned(),
        }),
    };
    let output = git(root, &["pull", flag, "--no-edit"])?;
    if output.status.success() {
        return Ok(());
    }
    let status = status_unlocked(root)?;
    if !status.conflicted.is_empty() {
        return Err(GitError::Conflict {
            message: mask_url_credentials(String::from_utf8_lossy(&output.stderr).trim()),
            files: status.conflicted,
        });
    }
    Err(error_from_stderr(&String::from_utf8_lossy(&output.stderr)))
}

pub fn status(root: &Path) -> Result<GitStatus, GitError> {
    with_lock(root, status_unlocked)
}

fn has_upstream(root: &Path) -> Result<bool, GitError> {
    Ok(git(root, &["rev-parse", "--verify", "@{upstream}"])?.status.success())
}

pub fn pull(root: &Path, method: &str) -> Result<GitStatus, GitError> {
    with_lock(root, |root| {
        ensure_no_conflicts(&status_unlocked(root)?)?;
        if has_upstream(root)? {
            pull_unlocked(root, method)?;
        }
        status_unlocked(root)
    })
}

pub fn sync(root: &Path, message: &str, method: &str, push: bool) -> Result<GitStatus, GitError> {
    with_lock(root, |root| {
        if !matches!(method, "merge" | "rebase") {
            return Err(GitError::Failed {
                message: "Unknown git sync method".to_owned(),
            });
        }
        let status = status_unlocked(root)?;
        ensure_no_conflicts(&status)?;
        if status.changed > 0 {
            checked(root, &["add", "-A", "--", ".", ":(exclude).aquilum/history"])?;
            checked(root, &["commit", "-m", message])?;
        }
        if has_upstream(root)? {
            pull_unlocked(root, method)?;
            if push {
                checked(root, &["push"])?;
            }
        }
        status_unlocked(root)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn masks_url_credentials_without_changing_other_text() {
        assert_eq!(
            mask_url_credentials("fatal: https://user:token@host/repo and ssh://user@host/repo https://host/path@name"),
            "fatal: https://***@host/repo and ssh://***@host/repo https://host/path@name"
        );
    }

    #[test]
    fn reads_porcelain_v2_including_renames_and_conflict_paths() {
        let text = "# branch.oid abc\0# branch.head main\0# branch.upstream origin/main\0# branch.ab +2 -3\0\
            1 .M N... 100644 100644 100644 abc abc ordinary.md\0\
            2 R. N... 100644 100644 100644 abc abc R100 new name.md\0old name.md\0\
            ? new\nfile.md\0\
            u UU N... 100644 100644 100644 100644 abc abc abc conflict name.md\0\
            ! ignored.md\0";
        let result = parse_status(text);
        assert_eq!(result.branch, "main");
        assert_eq!((result.ahead, result.behind, result.changed), (2, 3, 4));
        assert_eq!(result.conflicted, ["conflict name.md"]);
        assert_eq!(parse_status("# branch.head (detached)\n? file.md\n").changed, 1);
    }

    #[test]
    fn commit_message_formats_every_date_placeholder() {
        let now = LocalDateTime { year: 2026, month: 10, day: 8, hour: 9, minute: 2, second: 3 };
        assert_eq!(commit_message("{{date}} backup {{date}}", "YYYY-MM-DD HH:mm:ss", &now), "2026-10-08 09:02:03 backup 2026-10-08 09:02:03");
        assert_eq!(commit_message("vault backup: {{date}}", "DD.MM.YYYY", &now), "vault backup: 08.10.2026");
    }

    fn test_git(root: &Path, args: &[&str]) -> Output {
        let output = git(root, args).unwrap();
        assert!(output.status.success(), "{}", mask_url_credentials(&String::from_utf8_lossy(&output.stderr)));
        output
    }

    fn repositories() -> Option<(TempDir, PathBuf, PathBuf, PathBuf)> {
        let temporary = tempfile::tempdir().unwrap();
        match git(temporary.path(), &["--version"]) {
            Err(GitError::GitNotFound { .. }) => { eprintln!("git не найден: тест пропущен"); return None; }
            result => assert!(result.unwrap().status.success()),
        }
        let remote = temporary.path().join("remote.git");
        let local = temporary.path().join("local");
        let other = temporary.path().join("other");
        test_git(temporary.path(), &["init", "--bare", remote.to_str().unwrap()]);
        test_git(temporary.path(), &["init", "-b", "main", local.to_str().unwrap()]);
        configure(&local);
        fs::write(local.join("note.md"), "initial\n").unwrap();
        test_git(&local, &["add", "-A"]);
        test_git(&local, &["commit", "-m", "initial"]);
        test_git(&local, &["remote", "add", "origin", remote.to_str().unwrap()]);
        test_git(&local, &["push", "-u", "origin", "main"]);
        test_git(temporary.path(), &["clone", "-b", "main", remote.to_str().unwrap(), other.to_str().unwrap()]);
        configure(&other);
        Some((temporary, local, other, remote))
    }

    fn configure(root: &Path) {
        test_git(root, &["config", "user.name", "Git test"]);
        test_git(root, &["config", "user.email", "git-test@example.invalid"]);
        test_git(root, &["config", "commit.gpgsign", "false"]);
    }

    #[test]
    fn a_repository_without_remote_supports_local_backups_and_pull() {
        let temporary = tempfile::tempdir().unwrap();
        if matches!(git(temporary.path(), &["--version"]), Err(GitError::GitNotFound { .. })) {
            return;
        }
        test_git(temporary.path(), &["init", "-b", "main"]);
        configure(temporary.path());
        fs::write(temporary.path().join("note.md"), "local backup\n").unwrap();
        let result = sync(temporary.path(), "local backup", "merge", true).unwrap();
        assert_eq!(result.changed, 0);
        assert_eq!(pull(temporary.path(), "merge").unwrap().changed, 0);
        assert_eq!(String::from_utf8_lossy(&test_git(temporary.path(), &["log", "-1", "--format=%s"]).stdout).trim(), "local backup");
    }

    #[test]
    fn local_history_is_excluded_from_status_and_backup_commits() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path();
        test_git(root, &["init", "-b", "main"]);
        configure(root);
        fs::create_dir_all(root.join(".aquilum/history/note")).unwrap();
        fs::write(root.join(".aquilum/history/note/snapshot.md"), "history").unwrap();
        fs::write(root.join(".aquilum/plugins.json"), "{}").unwrap();
        fs::write(root.join("note.md"), "note").unwrap();
        assert_eq!(status(root).unwrap().changed, 2);
        assert_eq!(sync(root, "backup", "merge", false).unwrap().changed, 0);
        let files = test_git(root, &["ls-tree", "-r", "--name-only", "HEAD"]);
        assert_eq!(String::from_utf8_lossy(&files.stdout), ".aquilum/plugins.json\nnote.md\n");
        fs::write(root.join(".aquilum/history/note/second.md"), "more history").unwrap();
        assert_eq!(status(root).unwrap().changed, 0);
        sync(root, "no extra backup", "merge", false).unwrap();
        assert_eq!(String::from_utf8_lossy(&test_git(root, &["rev-list", "--count", "HEAD"]).stdout).trim(), "1");
    }

    #[test]
    fn status_counts_files_inside_untracked_directories() {
        let Some((_temporary, local, _other, _remote)) = repositories() else { return; };
        fs::create_dir(local.join("folder")).unwrap();
        fs::write(local.join("folder/a.md"), "a").unwrap();
        fs::write(local.join("folder/b.md"), "b").unwrap();
        assert_eq!(status(&local).unwrap().changed, 2);
    }

    #[test]
    fn mutex_serializes_operations_for_the_same_canonical_vault() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Barrier;
        let temporary = tempfile::tempdir().unwrap();
        let active = AtomicUsize::new(0);
        let barrier = Barrier::new(2);
        std::thread::scope(|scope| {
            for root in [temporary.path().to_owned(), temporary.path().join(".")] {
                let active = &active;
                let barrier = &barrier;
                scope.spawn(move || {
                    barrier.wait();
                    with_lock(&root, |_| {
                        assert_eq!(active.fetch_add(1, Ordering::SeqCst), 0);
                        std::thread::sleep(std::time::Duration::from_millis(20));
                        assert_eq!(active.fetch_sub(1, Ordering::SeqCst), 1);
                        Ok(())
                    }).unwrap();
                });
            }
        });
    }

    #[test]
    fn sync_commits_and_pushes_to_bare_remote_and_pull_downloads_changes() {
        for method in ["merge", "rebase"] {
            let Some((_temporary, local, other, remote)) = repositories() else { return; };
            fs::write(local.join("new note.md"), "backup\n").unwrap();
            let result = sync(&local, "backup message", method, true).unwrap();
            assert_eq!((result.changed, result.ahead, result.behind), (0, 0, 0));
            assert_eq!(String::from_utf8_lossy(&test_git(&remote, &["log", "main", "-1", "--format=%s"]).stdout).trim(), "backup message");
            pull(&other, method).unwrap();
            assert_eq!(fs::read_to_string(other.join("new note.md")).unwrap(), "backup\n");
            sync(&local, "no extra commit", method, true).unwrap();
            assert_eq!(String::from_utf8_lossy(&test_git(&local, &["rev-list", "--count", "HEAD"]).stdout).trim(), "2");
            fs::write(local.join("private.md"), "local\n").unwrap();
            assert_eq!(sync(&local, "local only", method, false).unwrap().ahead, 1);
            assert_eq!(String::from_utf8_lossy(&test_git(&remote, &["rev-list", "--count", "main"]).stdout).trim(), "2");
        }
    }

    #[test]
    fn conflicts_stop_without_cleanup_or_later_changes() {
        for method in ["merge", "rebase"] {
            let Some((_temporary, local, other, remote)) = repositories() else { return; };
            fs::write(other.join("note.md"), "remote edit\n").unwrap();
            sync(&other, "remote edit", method, true).unwrap();
            fs::write(local.join("note.md"), "local edit\n").unwrap();
            let error = sync(&local, "local edit", method, true).unwrap_err();
            assert!(matches!(error, GitError::Conflict { ref files, .. } if files == &["note.md"]));
            let conflicted = fs::read(local.join("note.md")).unwrap();
            assert!(String::from_utf8_lossy(&conflicted).contains("<<<<<<<"));
            assert_eq!(status(&local).unwrap().conflicted, ["note.md"]);
            assert!(local.join(".git").join(if method == "merge" { "MERGE_HEAD" } else { "rebase-merge" }).exists());
            assert!(matches!(pull(&local, method), Err(GitError::Conflict { .. })));
            assert!(matches!(sync(&local, "must not commit", method, true), Err(GitError::Conflict { .. })));
            assert_eq!(fs::read(local.join("note.md")).unwrap(), conflicted);
            let output = test_git(&remote, &["log", "main", "-1", "--format=%s"]);
            assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "remote edit");
        }
    }

    #[test]
    fn non_repository_and_auth_errors_keep_codes_and_masked_details() {
        let temporary = tempfile::tempdir().unwrap();
        if matches!(git(temporary.path(), &["--version"]), Err(GitError::GitNotFound { .. })) { return; }
        assert!(matches!(status(temporary.path()), Err(GitError::NotARepository { .. })));
        let error = error_from_stderr("fatal: Authentication failed for 'https://user:secret@host/repo'");
        let json = serde_json::to_value(error).unwrap();
        assert_eq!(json["code"], "auth");
        assert_eq!(json["details"]["message"], "fatal: Authentication failed for 'https://***@host/repo'");
    }
}
