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
    // Resolve their paths with `git rev-parse --git-path`. The refs/heads and refs/tags
    // folders catch a branch or tag file being written or packed away, so a commit is seen
    // whether HEAD's branch is a loose file or only in packed-refs.
    // Only existing paths are watched: cargo treats a missing one as changed and reruns
    // every build, and packed-refs exists only once refs have been packed.
    let git_paths_to_query = [
        "HEAD",
        "index",
        "packed-refs",
        "refs/heads",
        "refs/tags",
    ];

    let mut index_path = None;

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
                if abs_path.exists() {
                    println!("cargo:rerun-if-changed={}", abs_path.display());
                    if *path_key == "index" {
                        index_path = Some(abs_path);
                    }
                }
            }
        }
    }

    // Query git describe --long --tags --dirty --always
    let mut describe = Command::new("git");
    describe
        .args(["-C", manifest_dir.to_str().unwrap_or(".")])
        .args(["describe", "--long", "--tags", "--dirty", "--always"]);

    // `--dirty` refreshes and rewrites the index it reads. Give it a copy in OUT_DIR:
    // rewriting the watched .git/index would rerun the next build, and a build script
    // should not write into the source tree.
    if let (Some(index), Some(out_dir)) = (index_path, std::env::var_os("OUT_DIR")) {
        let copy = PathBuf::from(out_dir).join("git-index");
        if std::fs::copy(&index, &copy).is_ok() {
            describe.env("GIT_INDEX_FILE", &copy);
        }
    }

    if let Ok(out) = describe.output() {
        let desc = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if out.status.success() && !desc.is_empty() {
            println!("cargo:rustc-env=CAVE_GIT_DESCRIBE={desc}");
        }
    }
}
