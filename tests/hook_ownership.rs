//! Building this foundational dependency must never change a consumer's hooks.

use std::env;

use async_fs_io::{
    TempDir,
    ensure_dir,
    write_bytes,
};
use tokio::process::Command;

#[tokio::test]
async fn dependency_build_cannot_install_hooks_in_its_consumer() {
    let directory = TempDir::create(env::temp_dir())
        .await
        .expect("consumer fixture");
    let root = directory.path();
    ensure_dir(root.join("src"))
        .await
        .expect("source directory");
    write_bytes(root.join("src/lib.rs"), b"//! Consumer fixture.\n")
        .await
        .expect("source");
    let package_path = env!("CARGO_MANIFEST_DIR").replace('\\', "/");
    write_bytes(root.join("Cargo.toml"), format!(
        "[package]\nname='hook-owner-fixture'\nversion='0.1.0'\nedition='2024'\n[dependencies]\nasync-fs-io={{path='{package_path}'}}\n").as_bytes()).await.expect("manifest");
    for arguments in [
        vec!["init", "--quiet"],
        vec!["config", "core.hooksPath", "owned-hooks"],
    ] {
        let result = Command::new("git")
            .args(arguments)
            .current_dir(root)
            .output()
            .await
            .expect("prepare Git fixture");
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    let result = Command::new(env!("CARGO"))
        .args(["build", "--offline", "--manifest-path"])
        .arg(root.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", root.join("target"))
        .env("RHUSKY_REPOSITORY_ROOT", root)
        .env_remove("CI")
        .env_remove("GITHUB_ACTIONS")
        .current_dir(root)
        .output()
        .await
        .expect("build dependency outside CI");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let hooks = Command::new("git")
        .args(["config", "--get", "core.hooksPath"])
        .current_dir(root)
        .output()
        .await
        .expect("read consumer configuration");
    assert!(hooks.status.success());
    assert_eq!(
        String::from_utf8(hooks.stdout).expect("hooks path").trim(),
        "owned-hooks"
    );
}
