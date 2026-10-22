use std::ffi::{OsStr, OsString};
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
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

        // Initialize declared claude state using policy::prepare_host_state
        cave::policy::prepare_host_state(OsStr::new("claude"), &fake_home).unwrap();

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
        // Automatically inject set -eu into every test script
        let wrapped_script = format!("set -eu\n{}\necho '__SENTINEL_DONE__'", script);
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
fn test_harness_catches_failure() {
    let fixture = HermeticFixture::new();
    let output1 = fixture.run_cave(&[], "exit 1");
    assert!(!output1.status.success());
    assert!(!String::from_utf8_lossy(&output1.stdout).contains("__SENTINEL_DONE__"));

    let output2 = fixture.run_cave(&[], "false");
    assert!(!output2.status.success());
    assert!(!String::from_utf8_lossy(&output2.stdout).contains("__SENTINEL_DONE__"));

    let output3 = fixture.run_cave(&[], "if true; then exit 42; fi");
    assert_eq!(output3.status.code(), Some(42));
    assert!(!String::from_utf8_lossy(&output3.stdout).contains("__SENTINEL_DONE__"));
}

#[test]
fn test_outside_allowed_directories_are_hidden() {
    let fixture = HermeticFixture::new();
    let script = r#"
        if [ -e "$HOME/.ssh" ]; then echo "Failure: ~/.ssh is visible"; exit 1; fi
        if [ -e "$HOME/.config" ]; then echo "Failure: ~/.config is visible"; exit 1; fi
        if [ -e "$SIBLING" ]; then echo "Failure: sibling project is visible"; exit 1; fi
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
        if [ ! -L link_to_ssh ]; then echo "Failure: link_to_ssh is not a symlink"; exit 1; fi
        if [ -e link_to_ssh ]; then echo "Failure: link_to_ssh target resolved when it should dangle"; exit 1; fi
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
        if touch /usr/test_ro_file 2>/dev/null; then echo "Failure: /usr was writable"; exit 1; fi
        if touch /etc/test_ro_file 2>/dev/null; then echo "Failure: /etc was writable"; exit 1; fi
        if touch /var/test_ro_file 2>/dev/null; then echo "Failure: /var was writable"; exit 1; fi
        if touch "$HOME/.gitconfig" 2>/dev/null; then echo "Failure: ~/.gitconfig was writable"; exit 1; fi
        if touch "$EXTRA_RO/test_ro_file" 2>/dev/null; then echo "Failure: EXTRA_RO was writable"; exit 1; fi
    "#;

    let ro_arg = format!("{}", ro_dir.display());
    let output = fixture.run_cave(
        &["--ro-dir", &ro_arg],
        &format!("EXTRA_RO='{}' {}", ro_dir.display(), script),
    );
    fixture.assert_success_with_sentinel(&output);

    // Host verification: verify nothing was created on the host
    assert!(!ro_dir.join("test_ro_file").exists());
}

#[test]
fn test_read_write_working_dir_and_allowed_paths() {
    let fixture = HermeticFixture::new();
    let extra_dir = fixture._temp_dir.path().join("extra_rw");
    fs::create_dir_all(&extra_dir).unwrap();

    let script = r#"
        echo "working" > project_new.txt
        echo "extra" > "$EXTRA_RW/extra_new.txt"
        echo "cache" > "$HOME/.cache/new_cache.txt"
        echo "claude" > "$HOME/.claude/new_claude.txt"
    "#;

    let extra_arg = format!("{}", extra_dir.display());
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
        if [ -e "$HOST_MARKER" ]; then echo "Failure: host tmp marker is visible"; exit 1; fi
        echo "sandboxed" > /tmp/sandboxed_tmp.txt
        echo "scratch" > /var/tmp/sandboxed_scratch.txt
        if [ -e /run/docker.sock ]; then echo "Failure: /run/docker.sock is visible"; exit 1; fi
        if [ -e /var/run/docker.sock ]; then echo "Failure: /var/run/docker.sock is visible"; exit 1; fi
        UID_VAL="$(id -u)"
        if [ ! -d "/run/user/$UID_VAL" ]; then echo "Failure: /run/user/$UID does not exist"; exit 1; fi
        if [ -n "$(ls -A "/run/user/$UID_VAL")" ]; then echo "Failure: /run/user/$UID is not empty"; exit 1; fi
        PERMS="$(stat -c %a "/run/user/$UID_VAL")"
        if [ "$PERMS" != "700" ]; then echo "Failure: /run/user/$UID permissions are $PERMS (expected 700)"; exit 1; fi
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
        &format!("if [ ! -f '{}' ]; then echo 'Host marker missing with --dir /tmp'; exit 1; fi", fixture.host_tmp_marker.display()),
    );
    fixture.assert_success_with_sentinel(&output_tmp);

    // 2. --ro-dir . should make working dir read-only
    let output_ro_cwd = fixture.run_cave(
        &["--ro-dir", "."],
        "if touch write_attempt 2>/dev/null; then echo 'Write succeeded on read-only CWD'; exit 1; fi",
    );
    fixture.assert_success_with_sentinel(&output_ro_cwd);
}

#[test]
fn test_environment_variables_sanitization() {
    let fixture = HermeticFixture::new();
    let script = r#"
        if [ "$TEST_HOST_VAR" != "preserved_value" ]; then echo "Host variable not preserved"; exit 1; fi
        if [ -n "${SSH_AUTH_SOCK:-}" ]; then echo "SSH_AUTH_SOCK was not unset"; exit 1; fi
        if [ -n "${DBUS_SESSION_BUS_ADDRESS:-}" ]; then echo "DBUS_SESSION_BUS_ADDRESS was not unset"; exit 1; fi
        UID_VAL="$(id -u)"
        if [ "$XDG_RUNTIME_DIR" != "/run/user/$UID_VAL" ]; then echo "XDG_RUNTIME_DIR is wrong: $XDG_RUNTIME_DIR"; exit 1; fi
    "#;

    let output = fixture.run_cave(&[], script);
    fixture.assert_success_with_sentinel(&output);
}

#[test]
fn test_process_namespace_isolation() {
    let fixture = HermeticFixture::new();
    let host_pid = std::process::id();
    let script = format!(
        r#"
        # Host runner PID must not exist in sandbox /proc
        if [ -d "/proc/{host_pid}" ]; then
            echo "Failure: host runner PID {host_pid} is visible in sandbox /proc";
            exit 1;
        fi
        "#
    );

    let output = fixture.run_cave(&[], &script);
    fixture.assert_success_with_sentinel(&output);
}

#[test]
fn test_killing_cave_kills_agent_process() {
    let fixture = HermeticFixture::new();
    let cave_bin = env!("CARGO_BIN_EXE_cave");
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let unique_tag = format!("cave_canary_{}_{}", std::process::id(), nanos % 100_000);

    // Spawn cave running a background sleep carrying our unique tag
    let mut child = Command::new(cave_bin)
        .arg("sh")
        .arg("-c")
        .arg(format!("exec sleep 120 # {}", unique_tag))
        .current_dir(&fixture.working_dir)
        .env("HOME", &fixture.fake_home)
        .spawn()
        .expect("Failed to spawn cave");

    // Poll on host to verify the inner sleep process IS actually running
    let mut started = false;
    for _ in 0..50 {
        let pgrep = Command::new("pgrep")
            .arg("-f")
            .arg(&unique_tag)
            .output();
        if let Ok(out) = pgrep {
            started = out.status.success() && !out.stdout.is_empty();
            if started {
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(started, "Agent process with tag '{}' failed to start on host", unique_tag);

    // Kill the cave process
    child.kill().expect("Failed to kill cave child process");
    let _ = child.wait();

    // Give kernel a moment to reap namespace processes
    std::thread::sleep(Duration::from_millis(150));

    // Assert the host sees NO running process with this unique tag
    let pgrep_after = Command::new("pgrep")
        .arg("-f")
        .arg(&unique_tag)
        .output()
        .expect("Failed to run pgrep");
    assert!(
        !pgrep_after.status.success(),
        "Agent process with tag '{}' leaked on host after cave was killed!",
        unique_tag
    );
}

#[test]
fn test_non_utf8_arguments_pass_unmangled() {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;

        let fixture = HermeticFixture::new();
        let raw_bytes = vec![b'x', 0xff, b'y'];
        let non_utf8_arg = OsString::from_vec(raw_bytes.clone());

        let cave_bin = env!("CARGO_BIN_EXE_cave");
        let output = Command::new(cave_bin)
            .arg("python3")
            .arg("-c")
            .arg("import sys; sys.stdout.buffer.write(sys.argv[1].encode('utf-8', 'surrogateescape'))")
            .arg(non_utf8_arg)
            .current_dir(&fixture.working_dir)
            .env("HOME", &fixture.fake_home)
            .output()
            .expect("Failed to run cave with non-utf8 argument");

        assert!(output.status.success(), "Process failed: {}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(output.stdout, raw_bytes);
    }
}
