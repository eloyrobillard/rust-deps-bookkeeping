use clap::{arg, Command};

mod deprecated;
mod old;
mod package_json_parser;
mod registry;
mod types;

use deprecated::deprecated_monorepo;
use old::old_monorepo;
use package_json_parser::parse_package_json;
use registry::VersionObjectWithDeprecated;
use types::OldPkgDetails;

// LINK clap: https://docs.rs/clap/latest/clap/struct.Command.html
// LINK making CLIs in Rust: https://rust-cli.github.io/book/index.html
fn cli() -> Command {
    Command::new("debs")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("old")
                .about("Filter packages older than [YEARS (default: 4)]")
                .arg(arg!(-s --since <YEARS> "Minimum age of packages to be displayed").default_value("4"))
                .arg(arg!(-p --production "Add this option to exclusively show packages used in production").default_value("false"))
        )
        .subcommand(
            Command::new("deprecated")
                .about("Filter deprecated packages")
                .arg(arg!(-p --production "Add this option to exclusively show packages used in production").default_value("false")))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = cli().get_matches();

    match matches.subcommand() {
        Some(("old", sub_matches)) => {
            let since = sub_matches
                .get_one::<String>("since")
                .map(|s| s.as_str())
                .expect("defaulted in clap");

            let production_pkgs_only = sub_matches
                .get_one::<bool>("production")
                .expect("defaulted in clap");

            let pkg_json = parse_package_json(None)?;

            if pkg_json.workspaces.is_none() {
                finalize_old(
                    since.parse::<u32>().unwrap_or(4),
                    *production_pkgs_only,
                    None,
                    None,
                    false
                )
                .await?
            } else {
                for workspace in pkg_json.workspaces.unwrap() {
                    println!("{} old packages:", workspace);
                    println!();

                    finalize_old(
                        since.parse::<u32>().unwrap_or(4),
                        *production_pkgs_only,
                        Some(&workspace),
                        None,
                        workspace == "frontend"
                    )
                    .await?
                }
            }
        }
        Some(("deprecated", sub_matches)) => {
            let production_pkgs_only = sub_matches
                .get_one::<bool>("production")
                .expect("defaulted in clap");

            let pkg_json = parse_package_json(None)?;

            if pkg_json.workspaces.is_none() {
                finalize_deprecated(*production_pkgs_only, None, None, false).await?
            } else {
                for workspace in pkg_json.workspaces.unwrap() {
                    println!("{} deprecated packages:", workspace);
                    println!();

                    finalize_deprecated(*production_pkgs_only, Some(&workspace), None, workspace == "frontend").await?
                }
            }
        }
        _ => unreachable!(),
    }

    Ok(())
}

// TODO よりいい関数名がないか？
async fn finalize_old(
    since: u32,
    production_pkgs_only: bool,
    path_pkg_json: Option<&str>,
    path_lock_json: Option<&str>,
    in_frontend: bool
) -> Result<(), Box<dyn std::error::Error>> {
    let old_pkgs = old_monorepo(since, path_pkg_json, path_lock_json, in_frontend).await?;

    if production_pkgs_only {
        output_old(&old_pkgs.0, None);

        let num_old_pkgs = old_pkgs.0.len();

        println!("Total: {num_old_pkgs} old production dependencies");
        println!();
    } else {
        output_old(&old_pkgs.0, Some("Production Dependencies:"));
        output_old(&old_pkgs.1, Some("Development Dependencies:"));

        let num_old_prods = old_pkgs.0.len();

        let num_old_devs = old_pkgs.1.len();

        println!("Total: {num_old_prods} old dependencies, {num_old_devs} old dev dependencies");
        println!();
    }

    Ok(())
}

// TODO よりいい関数名がないか？
/// # Arguments
///
/// * `json_path` - An optional &str holding the path to the package.json and package-lock.json. Needs to end with '/'.
///
async fn finalize_deprecated(
    production_pkgs_only: bool,
    path_pkg_json: Option<&str>,
    path_lock_json: Option<&str>,
    in_frontend: bool
) -> Result<(), Box<dyn std::error::Error>> {
    let (prod_depr, dev_depr) = deprecated_monorepo(path_pkg_json, path_lock_json, in_frontend).await?;

    if production_pkgs_only {

        output_deprecated(&prod_depr, None);

        let num_depr_pkgs = prod_depr.len();

        println!("Total: {num_depr_pkgs} deprecated production dependencies");
        println!();
    } else {

        output_deprecated(&prod_depr, Some("Production Dependencies:"));
        output_deprecated(&dev_depr, Some("Development Dependencies:"));

        let num_depr_prods = prod_depr.len();

        let num_depr_devs = dev_depr.len();

        println!(
            "Total: {num_depr_prods} deprecated dependencies, {num_depr_devs} deprecated dev dependencies"
        );
        println!();
    };

    Ok(())
}

// Utils

fn output_old(pkgs: &Vec<OldPkgDetails>, tag_line: Option<&str>) {
    if pkgs.is_empty() {
        return
    }

    if tag_line.is_some() {
        println!("{}", tag_line.unwrap());
        println!();
    }

    for OldPkgDetails {
        name,
        version,
        date_version,
        age_version,
        latest_version,
        date_latest_version,
        age_latest_version,
        ..
    } in pkgs
    {
        let simple_date_version = date_version.format("%d/%m/%Y");

        println!("{name}@{version} ({simple_date_version})");

        let age_diff = age_version - age_latest_version;

        let simple_date_latest_version = date_latest_version.format("%d/%m/%Y");

        println!("  -> {age_version} years old, {age_diff} older than latest");
        println!("      -> Latest @{latest_version} ({simple_date_latest_version})");
        println!();
    }
}

fn output_deprecated(pkgs: &Vec<VersionObjectWithDeprecated>, tag_line: Option<&str>) {
    if pkgs.is_empty() {
        return
    }

    if tag_line.is_some() {
        println!("{}", tag_line.unwrap());
        println!();
    }

    for VersionObjectWithDeprecated { name, version, .. } in pkgs {
        println!("{name}@{version}");
    }
    println!();
}
