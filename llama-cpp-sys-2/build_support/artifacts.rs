use std::ffi::OsStr;
use std::path::{Path, PathBuf};

pub(crate) fn profile_dir(out_dir: &Path) -> Result<&Path, &'static str> {
    // Cargo 1.100 inserts a package-name directory before the build hash.
    // The profile's `build` ancestor is stable across both layouts.
    out_dir
        .ancestors()
        .find(|directory| directory.file_name() == Some(OsStr::new("build")))
        .and_then(Path::parent)
        .ok_or("OUT_DIR is not inside a Cargo profile build directory")
}

pub(crate) fn legacy_test_dir(profile_dir: &Path) -> Option<PathBuf> {
    let directory = profile_dir.join("deps");
    directory.is_dir().then_some(directory)
}

#[cfg(test)]
mod tests {
    use super::{legacy_test_dir, profile_dir};
    use std::path::Path;

    #[test]
    fn resolves_native_shared_library_destinations_across_cargo_layouts() {
        for (out_dir, expected) in [
            (
                "target/debug/build/llama-cpp-sys-2-0123456789abcdef/out",
                "target/debug",
            ),
            (
                "target/release/build/llama-cpp-sys-2/0123456789abcdef/out",
                "target/release",
            ),
            (
                "target/aarch64-unknown-linux-gnu/custom/build/llama-cpp-sys-2/0123456789abcdef/out",
                "target/aarch64-unknown-linux-gnu/custom",
            ),
            (
                "workspace/build/project/target/release/build/llama-cpp-sys-2/0123456789abcdef/out",
                "workspace/build/project/target/release",
            ),
        ] {
            assert_eq!(profile_dir(Path::new(out_dir)), Ok(Path::new(expected)));
        }
    }

    #[test]
    fn rejects_output_outside_a_cargo_build_directory() {
        assert!(profile_dir(Path::new("target/release/llama/out")).is_err());
    }

    #[test]
    fn legacy_runtime_directory_follows_cargo_directory_lifecycle() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let profile =
            std::env::temp_dir().join(format!("llama-artifacts-{}-{unique}", std::process::id()));
        std::fs::create_dir(&profile).unwrap();
        let deps = profile.join("deps");
        assert_eq!(legacy_test_dir(&profile), None);
        std::fs::create_dir(&deps).unwrap();
        assert_eq!(legacy_test_dir(&profile), Some(deps.clone()));
        std::fs::remove_dir(&deps).unwrap();
        assert_eq!(legacy_test_dir(&profile), None);
        std::fs::remove_dir(profile).unwrap();
    }
}
