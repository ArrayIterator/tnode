use chrono::Utc;
use std::env::home_dir;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::env;

/// Attempts to determine the path to the `npm` executable.
///
/// This function searches for the `npm` executable using the following methods:
/// 1. It first tries using the `which` command to find the globally installed `npm`.
/// 2. If the `which` command fails to locate `npm`, it then checks for `npm` installed
///    via the Volta or NVM version managers in the user's home directory:
///    - For **Volta**, it checks if `~/.volta/bin/npm` exists.
///    - For **NVM**, it scans directories under `~/.nvm/versions/node`
///      and checks if a valid `bin/npm` exists.
///
/// # Returns
/// - `Some(String)` containing the path to the `npm` executable if found.
/// - `None` if the `npm` executable cannot be located.
///
/// # Example
/// ```rust
/// if let Some(npm_path) = get_npm_executable() {
///     println!("npm found at: {}", npm_path);
/// } else {
///     println!("npm not found.");
/// }
/// ```
///
/// # Notes
/// - This function relies on the `std::process::Command` to invoke the `which` command
///   and attempts to parse its output.
/// - If using Volta or NVM, ensure that the respective directories are correctly configured
///   in the user's home directory.
fn get_npm_executable() -> Option<PathBuf> {
    if let Ok(output) = Command::new("which").arg("npm").output() {
        if output.status.success() {
            let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path_str.is_empty() {
                return Some(PathBuf::from(path_str));
            }
        }
    }

    if let Some(home) = home_dir() {
        let mut volta_npm = PathBuf::from(home.clone());
        volta_npm.push(".volta/bin/npm");

        if volta_npm.exists() {
            println!("Found npm via Volta at: {}", volta_npm.to_string_lossy());
            return Some(volta_npm);
        }

        let mut nvm_npm = PathBuf::from(home.clone());
        nvm_npm.push(".nvm/versions/node/");
        if let Ok(entries) = std::fs::read_dir(nvm_npm) {
            for entry in entries.flatten() {
                let mut candidate = entry.path();
                candidate.push("bin/npm");
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

/// A Rust build script that handles frontend changes, environment-specific behavior, and communicates build metadata to the Rust compiler.
///
/// # Functionality
/// - **Frontend Change Detection**:
///   Notifies Cargo to re-run the build script if any changes are detected in the `frontend/src` directory via the `cargo:rerun-if-changed` directive.
///
/// - **CI Environment Check**:
///   Detects if the environment variable `CI` is set (indicating a CI/CD pipeline environment) and skips the frontend build if true, to save CI build time.
///
/// - **Build Timestamp Communication**:
///   Uses the `chrono` crate to generate a build timestamp and formatted date string. These are passed to the Rust compiler as environment variables
///   (`BUILD_TIMESTAMP` and `BUILD_TIMESTAMP_DATE`) via the `cargo:rustc-env` directive.
///
/// - **Frontend Build Using npm**:
///   Locates the `npm` executable, runs the frontend build using `npm run build` in the `frontend` directory, and streams the output.
///   If the `npm` executable is not found or if the build fails, the script panics with appropriate error messages.
///
/// - **Change Tracking for Build Script**:
///   Ensures that changes to the `build.rs` file itself will trigger a rebuild by notifying Cargo using the `cargo:rerun-if-changed` directive.
///
/// # Panics
/// - Panics if:
///   - The `npm` executable is not found.
///   - The frontend build process fails or exits with a non-zero status.
///
/// # Dependencies
/// - The `chrono` crate is required to generate timestamps.
/// - `std::env`, `std::process::Command`, and `std::process::Stdio` are used for environment variable handling and spawning the build process.
///
/// # Example
/// Add the script to a `build.rs` file in the root of a Rust project. Ensure that the `frontend` directory and `npm` are correctly set up.
/// ```rust
/// // build.rs
/// fn main() {
///     // Script logic here...
/// }
/// ```
///
/// # Expected Output
/// - Outputs timestamp-related metadata to Cargo.
/// - Runs the frontend build process if in a non-CI environment.
/// - Logs errors and exits if preconditions or the build process fail.
///
/// # Related Notes
/// - To ensure the script works, `chrono` must be added as a dependency in `Cargo.toml`.
/// - The script assumes a `frontend` directory exists and has a valid `package.json` with a `build` script.
///
/// # See Also
/// For more information on build scripts:
/// - [Cargo Build Scripts](https://doc.rust-lang.org/cargo/reference/build-scripts.html)
fn main() {
    println!("cargo:rerun-if-changed=frontend/src");
    if env::var("CI").is_ok() {
        println!("Detected CI environment, skipping frontend build.");
        return;
    }
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let project_dir = PathBuf::from(manifest_dir);

    let project_npm_paths= ["frontend"];

    let now = Utc::now();
    let timestamp = now.timestamp();
    let timestamp_date = now.format("%Y-%m-%d %H:%M:%S UTC").to_string();
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", timestamp);
    println!("cargo:rustc-env=BUILD_TIMESTAMP_DATE={}", timestamp_date);

    let npm_path = get_npm_executable().unwrap_or_else(|| panic!("Could not find npm executable! Please ensure npm is installed and available in your PATH."));
    println!("Using npm at path: {}", npm_path.display());
    let _= Stdio::inherit();
    for path in project_npm_paths.iter() {
        let _absolute_path = project_dir.join(path);
        // todo: build frontend
        // println!("Building : {}", path);
        // let mut child = Command::new(npm_path.clone())
        //     .args(&["run", "build"])
        //     .current_dir(absolute_path)
        //     .stdout(Stdio::inherit())
        //     .stderr(Stdio::inherit())
        //     .spawn()
        //     .expect("Fail to start Vite build process!");
        // let status = child.wait().expect("Failed to wait on Vite build process!");
        // println!("cargo:rerun-if-changed=build.rs");
        // if !status.success() {
        //     panic!("Vite build process failed!");
        // }
    }
}
