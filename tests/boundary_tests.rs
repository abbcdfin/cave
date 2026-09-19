use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::TempDir;

struct HermeticFixture {
    _temp_dir: TempDir,
    fake_home: PathBuf,
    working_dir: PathBuf,
    sibling_project: PathBuf,
    host_tmp_marker: PathBuf,
    fake_claude_bin: PathBuf,
}

impl HermeticFixture {
    fn new() -> Self {
        cave::bwrap::check_bwrap_available().expect("bwrap is required for boundary tests");

        let temp_dir = tempfile::tempdir().expect("Failed to create tempdir for fixture");
        let root = temp_dir.path();

        let fake_home = root.join("fake_home");
        let working_dir = root.join("working_dir");
        let sibling_project = root.join("sibling_project");

        fs::create_dir_all(&fake_home).unwrap();
        fs::create_dir_all(&working_dir).unwrap();
        fs::create_dir_all(&sibling_project).unwrap();

        // Fake home contents
        let ssh_dir = fake_home.join(".ssh");
        fs::create_dir_all(&ssh_dir).unwrap();
        fs::write(ssh_dir.join("id_rsa"), "SECRET_KEY").unwrap();

        let config_dir = fake_home.join(".config").join("gcloud");
        fs::create_dir_all(&config_dir).unwrap();
        fs::write(config_dir.join("credentials"), "TOKEN").unwrap();

        let claude_dir = fake_home.join(".claude");
        fs::create_dir_all(&claude_dir).unwrap();
        fs::write(claude_dir.join("settings.json"), "{}").unwrap();
        fs::write(fake_home.join(".claude.json"), "{}").unwrap();

        let cache_dir = fake_home.join(".cache");
        fs::create_dir_all(&cache_dir).unwrap();
        fs::write(cache_dir.join("cache.txt"), "cached data").unwrap();

        fs::write(fake_home.join(".gitconfig"), "[user]\n  name = Test User\n").unwrap();

        // Working dir contents
        fs::write(working_dir.join("project_file.txt"), "hello project").unwrap();
        // Symlink pointing to outside sensitive directory
        symlink(&ssh_dir, working_dir.join("link_to_ssh")).unwrap();

        // Fake claude agent binary (symlink to sh so it can execute scripts while triggering claude profile)
        let fake_claude_bin = working_dir.join("claude");
        symlink("/bin/sh", &fake_claude_bin).unwrap();

        // Sibling project
        fs::write(sibling_project.join("secret_sibling.txt"), "sibling data").unwrap();

        // Host /tmp marker
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let marker_name = format!("cave_test_host_tmp_marker_{}_{}.txt", std::process::id(), nanos);
        let host_tmp_marker = PathBuf::from("/tmp").join(marker_name);
        fs::write(&host_tmp_marker, "host marker").unwrap();

        HermeticFixture {
            _temp_dir: temp_dir,
            fake_home,
            working_dir,
            sibling_project,
            host_tmp_marker,
            fake_claude_bin,
        }
    }

    fn run_cave(&self, args: &[&str], script: &str) -> std::process::Output {
        self.run_cave_agent("sh", args, script)
    }

    fn run_cave_agent(&self, agent: &str, args: &[&str], script: &str) -> std::process::Output {
        let cave_bin = env!("CARGO_BIN_EXE_cave");
        let mut cmd = Command::new(cave_bin);
        let wrapped_script = format!("{}\necho '__SENTINEL_DONE__'", script);
        cmd.args(args)
            .arg(agent)
            .arg("-c")
            .arg(&wrapped_script)
            .current_dir(&self.working_dir)
            .env("HOME", &self.fake_home)
            .env("TEST_HOST_VAR", "preserved_value")
            .env("SSH_AUTH_SOCK", "/tmp/fake_ssh_auth_sock")
            .env("DBUS_SESSION_BUS_ADDRESS", "unix:path=/tmp/fake_dbus");

        cmd.output().expect("Failed to execute cave binary")
    }

    fn assert_success_with_sentinel(&self, output: &std::process::Output) {
        assert!(
            output.status.success(),
            "Process failed. Exit code: {:?}\nStdout: {}\nStderr: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("__SENTINEL_DONE__"),
            "Command did not execute through to sentinel! Stdout: {}\nStderr: {}",
            stdout,
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

impl Drop for HermeticFixture {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.host_tmp_marker);
    }
}

#[test]
fn test_outside_allowed_directories_are_hidden() {
    let fixture = HermeticFixture::new();
    let script = r#"
        set -e
        # ~/.ssh and ~/.config must not exist
        [ ! -e "$HOME/.ssh" ]
        [ ! -e "$HOME/.config" ]
        # Sibling project path must not exist
        [ ! -e "$SIBLING" ]
    "#;

    let output = fixture.run_cave(
        &[],
        &format!("SIBLING='{}' {}", fixture.sibling_project.display(), script),
    );
    fixture.assert_success_with_sentinel(&output);
}

#[test]
fn test_symlinks_pointing_outside_dangle() {
    let fixture = HermeticFixture::new();
    let script = r#"
        # link_to_ssh points to ~/.ssh on host, which is hidden in the sandbox, so link must dangle
        [ -L link_to_ssh ]
        [ ! -e link_to_ssh ]
    "#;

    let output = fixture.run_cave(&[], script);
    fixture.assert_success_with_sentinel(&output);
}

#[test]
fn test_read_only_boundaries() {
    let fixture = HermeticFixture::new();
    let ro_dir = fixture._temp_dir.path().join("extra_ro");
    fs::create_dir_all(&ro_dir).unwrap();

    let script = r#"
        # Writes under /usr, /etc, /var, ~/.gitconfig, and extra --ro-dir must fail
        ! touch /usr/new_file 2>/dev/null
        ! touch /etc/new_file 2>/dev/null
        ! touch /var/new_file 2>/dev/null
        ! touch "$HOME/.gitconfig" 2>/dev/null
        ! touch "$EXTRA_RO/new_file" 2>/dev/null
    "#;

    let ro_arg = format!("{}", ro_dir.display());
    let output = fixture.run_cave(
        &["--ro-dir", &ro_arg],
        &format!("EXTRA_RO='{}' {}", ro_dir.display(), script),
    );
    fixture.assert_success_with_sentinel(&output);
}

#[test]
fn test_read_write_working_dir_and_allowed_paths() {
    let fixture = HermeticFixture::new();
    let extra_dir = fixture._temp_dir.path().join("extra_rw");
    fs::create_dir_all(&extra_dir).unwrap();

    let script = r#"
        set -e
        echo "working" > project_new.txt
        echo "extra" > "$EXTRA_RW/extra_new.txt"
        echo "cache" > "$HOME/.cache/new_cache.txt"
        echo "claude" > "$HOME/.claude/new_claude.txt"
    "#;

    let extra_arg = format!("{}", extra_dir.display());
    // Use the fake claude agent binary to activate the claude profile
    let claude_agent = format!("{}", fixture.fake_claude_bin.display());
    let output = fixture.run_cave_agent(
        &claude_agent,
        &["--dir", &extra_arg],
        &format!("EXTRA_RW='{}' {}", extra_dir.display(), script),
    );
    fixture.assert_success_with_sentinel(&output);

    // Verify written files actually exist on host
    assert_eq!(
        fs::read_to_string(fixture.working_dir.join("project_new.txt")).unwrap(),
        "working\n"
    );
    assert_eq!(
        fs::read_to_string(extra_dir.join("extra_new.txt")).unwrap(),
        "extra\n"
    );
    assert_eq!(
        fs::read_to_string(fixture.fake_home.join(".cache").join("new_cache.txt")).unwrap(),
        "cache\n"
    );
    assert_eq!(
        fs::read_to_string(fixture.fake_home.join(".claude").join("new_claude.txt")).unwrap(),
        "claude\n"
    );
}

#[test]
fn test_private_temporary_and_run_filesystems() {
    let fixture = HermeticFixture::new();
    let script = r#"
        set -e
        # Host /tmp marker must be invisible
        [ ! -e "$HOST_MARKER" ]
        # Writing to /tmp or /var/tmp succeeds inside sandbox
        echo "sandboxed" > /tmp/sandboxed_tmp.txt
        echo "scratch" > /var/tmp/sandboxed_scratch.txt
        # Sockets must be absent
        [ ! -e /run/docker.sock ]
        [ ! -e /var/run/docker.sock ]
        # /run/user/$UID exists, empty, mode 0700
        UID_VAL="$(id -u)"
        [ -d "/run/user/$UID_VAL" ]
        [ -z "$(ls -A /run/user/$UID_VAL)" ]
        PERMS="$(stat -c %a "/run/user/$UID_VAL")"
        [ "$PERMS" = "700" ]
    "#;

    let output = fixture.run_cave(
        &[],
        &format!("HOST_MARKER='{}' {}", fixture.host_tmp_marker.display(), script),
    );
    fixture.assert_success_with_sentinel(&output);

    // Host /tmp and /var/tmp must NOT have the sandbox files
    assert!(!Path::new("/tmp/sandboxed_tmp.txt").exists());
    assert!(!Path::new("/var/tmp/sandboxed_scratch.txt").exists());
}

#[test]
fn test_overrides_dir_tmp_and_ro_working_dir() {
    let fixture = HermeticFixture::new();

    // 1. --dir /tmp should expose the host /tmp marker
    let output_tmp = fixture.run_cave(
        &["--dir", "/tmp"],
        &format!("[ -f '{}' ]", fixture.host_tmp_marker.display()),
    );
    fixture.assert_success_with_sentinel(&output_tmp);

    // 2. --ro-dir . should make working dir read-only
    let output_ro_cwd = fixture.run_cave(
        &["--ro-dir", "."],
        "! touch write_attempt 2>/dev/null",
    );
    fixture.assert_success_with_sentinel(&output_ro_cwd);
}

#[test]
fn test_environment_variables_sanitization() {
    let fixture = HermeticFixture::new();
    let script = r#"
        set -e
        # Host variable passes through
        [ "$TEST_HOST_VAR" = "preserved_value" ]
        # Hidden sockets variables are unset
        [ -z "$SSH_AUTH_SOCK" ]
        [ -z "$DBUS_SESSION_BUS_ADDRESS" ]
        # XDG_RUNTIME_DIR is /run/user/$UID
        [ "$XDG_RUNTIME_DIR" = "/run/user/$(id -u)" ]
    "#;

    let output = fixture.run_cave(&[], script);
    fixture.assert_success_with_sentinel(&output);
}

#[test]
fn test_process_namespace_isolation() {
    let fixture = HermeticFixture::new();
    let script = r#"
        # In an unshared PID namespace, PID 1 is the sandbox root process
        # and host PIDs are invisible.
        [ -d /proc/1 ]
    "#;

    let output = fixture.run_cave(&[], script);
    fixture.assert_success_with_sentinel(&output);
}
