use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    // Rebuild triggers: once build.rs prints any cargo:rerun-if-changed,
    // cargo reruns it only when a listed path changes, so the list must be complete.
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=build.rs");

    let manifest_dir = match std::env::var("CARGO_MANIFEST_DIR") {
        Ok(dir) => PathBuf::from(dir),
        Err(_) => return,
    };

    // Trust git only if `git rev-parse --show-toplevel` equals CARGO_MANIFEST_DIR.
    // Otherwise a source copy unpacked inside some other checkout would report
    // that other repo's commit.
    let toplevel_out = Command::new("git")
        .args(["-C", manifest_dir.to_str().unwrap_or(".")])
        .args(["rev-parse", "--show-toplevel"])
        .output();

    let toplevel_str = match toplevel_out {
        Ok(ref out) if out.status.success() => String::from_utf8_lossy(&out.stdout).trim().to_string(),
        _ => return,
    };

    let toplevel_path = PathBuf::from(toplevel_str);
    let canon_manifest = std::fs::canonicalize(&manifest_dir).unwrap_or_else(|_| manifest_dir.clone());
    let canon_toplevel = std::fs::canonicalize(&toplevel_path).unwrap_or_else(|_| toplevel_path.clone());

    if canon_manifest != canon_toplevel {
        return;
    }

    // Watch git files that change on a commit or a tag.
    // Resolve their paths with `git rev-parse --git-path`:
    // HEAD, index, packed-refs, refs/tags, and the branch ref that HEAD points to.
    let git_paths_to_query = [
        "HEAD",
        "index",
        "packed-refs",
        "refs/tags",
    ];

    for path_key in &git_paths_to_query {
        let Ok(out) = Command::new("git")
            .args(["-C", manifest_dir.to_str().unwrap_or(".")])
            .args(["rev-parse", "--git-path", path_key])
            .output()
        else {
            continue;
        };

        if out.status.success() {
            let git_path_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !git_path_str.is_empty() {
                let p = Path::new(&git_path_str);
                let abs_path = if p.is_absolute() {
                    p.to_path_buf()
                } else {
                    manifest_dir.join(p)
                };
                println!("cargo:rerun-if-changed={}", abs_path.display());
            }
        }
    }

    // Also watch the branch ref that HEAD points to (e.g. refs/heads/main)
    let sym_out = Command::new("git")
        .args(["-C", manifest_dir.to_str().unwrap_or(".")])
        .args(["symbolic-ref", "-q", "HEAD"])
        .output();

    if let Ok(sym_out) = sym_out {
        let ref_name = String::from_utf8_lossy(&sym_out.stdout).trim().to_string();
        if sym_out.status.success() && !ref_name.is_empty() {
            let ref_path_out = Command::new("git")
                .args(["-C", manifest_dir.to_str().unwrap_or(".")])
                .args(["rev-parse", "--git-path", &ref_name])
                .output();

            if let Ok(ref_path_out) = ref_path_out {
                let ref_path_str = String::from_utf8_lossy(&ref_path_out.stdout).trim().to_string();
                if ref_path_out.status.success() && !ref_path_str.is_empty() {
                    let p = Path::new(&ref_path_str);
                    let abs_path = if p.is_absolute() {
                        p.to_path_buf()
                    } else {
                        manifest_dir.join(p)
                    };
                    println!("cargo:rerun-if-changed={}", abs_path.display());
                }
            }
        }
    }

    // Query git describe --long --tags --dirty --always
    let desc_out = Command::new("git")
        .args(["-C", manifest_dir.to_str().unwrap_or(".")])
        .args(["describe", "--long", "--tags", "--dirty", "--always"])
        .output();

    if let Ok(out) = desc_out {
        let desc = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if out.status.success() && !desc.is_empty() {
            println!("cargo:rustc-env=CAVE_GIT_DESCRIBE={desc}");
        }
    }
}
