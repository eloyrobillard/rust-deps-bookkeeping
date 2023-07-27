use std::path::PathBuf;

use clap::{arg, Command};

mod deprecated;
mod old;
mod package_json;
mod registry;
mod types;

use deprecated::get_deprecated_packages;
use old::get_old_packages;
use package_json::parse_package_json;

/// Initializes a command line interface using the [`clap`] module.
///
/// You can find more details on making CLIs in Rust [here](https://rust-cli.github.io/book/index.html).
fn cli() -> Command {
    Command::new("debs")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("old")
                .about("Filter packages older than [YEARS (default: 4)]")
                // this is an interesting example of what can be done with Rust macros
                // things passed to the `arg!` macro represent both sue of the command
                // as well as what gets printed out in the help menu
                .arg(arg!(-s --since <YEARS> "Minimum age of packages to be displayed").default_value("4"))
                // no parameter is specified (e.g. no "<PROD>") so this is a boolean
                .arg(arg!(-p --production "Add this option to exclusively show packages used in production").default_value("false"))
                .arg(arg!(--path <PATH> "Specify the path to, but not including, the root package.json").default_value(""))
            )
            .subcommand(
                Command::new("deprecated")
                .about("Filter deprecated packages")
                .arg(arg!(-p --production "Add this option to exclusively show packages used in production").default_value("false"))
                .arg(arg!(--path <PATH> "Specify the path to the root package.json").default_value(""))
            )
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = cli().get_matches();

    match matches.subcommand() {
        Some((command_name, sub_matches)) => {
            let production_pkgs_only = sub_matches
                .get_one::<bool>("production")
                .expect("defaulted in clap");

            let path = sub_matches
                .get_one::<String>("path")
                .expect("defaulted in clap");

            let mut path = PathBuf::from(path);

            if let false = path.is_absolute() {
                // use relative path if fails to read cwd
                // then again, if reading the cwd fails, you probably have other things to worry about
                path = std::env::current_dir().unwrap_or_default().join(path);
            };

            // root package.json
            let pkg_json = parse_package_json(&path)?;

            match command_name {
                "deprecated" => {
                    match pkg_json.workspaces {
                        None => panic!("No workspaces found in package.json. Cannot read the path to the necessary dependency information."),
                        // `*` operator is used to deref a reference, here: &bool -> bool (same use as in C/C++)
                        // `&` creates an immutable reference to `workspaces`. There can be any amount of immutable refs at a single point in time.
                        // `&mut` would create a mutable reference. There can only be a single mut ref to something at a single point in time.
                        Some(workspaces) => println!("{}", get_deprecated_packages(&path, &workspaces, !(*production_pkgs_only)).await),
                    }
                }
                "old" => {
                    let since = sub_matches
                        .get_one::<String>("since")
                        .expect("defaulted in clap");

                    let since = since.parse::<u32>().unwrap_or(4);

                    match pkg_json.workspaces {
                        None => panic!("No workspaces found in package.json. Cannot read the path to the necessary dependency information."),
                        // passing stdout as `writer` to `old`, to showcase a different way to handle testing output than returning a String
                        // inside `old`'s test, we pass a Vec<u8> instead of stdout to collect the output
                        Some(workspaces) => {get_old_packages(since, &path, &workspaces, &mut std::io::stdout(), !(*production_pkgs_only)).await?;},
                    }
                }
                _ => unreachable!(),
            }
        }
        _ => unreachable!(),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    // LINK CLIをテストする：　https://rust-cli.github.io/book/tutorial/testing.html
    // LINK rustlingsのcliテスト： https://github.com/rust-lang/rustlings/blob/main/tests/integration_tests.rs
    //! ``` rust
    //! #[test]
    //! fn run_single_compile_success() {
    //!     Command::cargo_bin("rustlings")
    //!         .unwrap()
    //!         .args(&["run", "compSuccess"])
    //!         .current_dir("tests/fixture/success/") // --path を使わなくていい
    //!         .assert()
    //!         .success();
    //! }
    //! ```

    use std::collections::HashMap;
    use std::error::Error;
    // LINK https://doc.rust-lang.org/std/path/struct.PathBuf.html#impl-Default-for-PathBuf
    use std::path::PathBuf;
    // LINK https://doc.rust-lang.org/std/process/struct.Command.html#method.args
    use std::process::Command; // Run programs

    use assert_cmd::prelude::*; // Add methods on commands
    use once_cell::sync::Lazy;
    use predicates::prelude::*; // Used for writing assertions

    static TARGETS: Lazy<HashMap<&str, HashMap<&str, &str>>> = Lazy::new(|| {
        HashMap::from([(
            "macos",
            HashMap::from([
                ("x86_64", "x86_64-apple-darwin"),
                ("aarch64", "aarch64-apple-darwin"),
            ]),
        )])
    });

    static BIN_PATH: Lazy<PathBuf> = Lazy::new(|| {
        std::env::current_dir()
            .unwrap_or_default()
            .join(["builds", get_target(), "debs"].iter().collect::<PathBuf>())
    });

    static PKG_JSON_PATH: &str = "./test-assets/monorepo";

    #[test]
    fn runs_without_arguments() -> Result<(), Box<dyn Error>> {
        let path = BIN_PATH.to_str();

        assert!(path.is_some());

        let mut cmd = Command::cargo_bin(path.unwrap())?;

        // no arguments: should return error with help and exit gracefully
        cmd.assert()
            .failure()
            .stderr(predicate::str::contains("Usage: debs <COMMAND>"));

        Ok(())
    }

    #[test]
    fn runs_old() -> Result<(), Box<dyn Error>> {
        let path = BIN_PATH.to_str();

        assert!(path.is_some());

        let mut cmd = Command::cargo_bin(path.unwrap())?;

        cmd.arg("old").current_dir(PKG_JSON_PATH);
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("[common/] old packages:"));

        Ok(())
    }

    // UTILS

    fn get_target() -> &'static str {
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;

        TARGETS.get(os).unwrap().get(arch).unwrap()
    }
}
