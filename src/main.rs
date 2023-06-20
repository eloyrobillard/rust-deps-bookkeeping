use clap::{arg, Command};

mod deprecated;
mod old;
mod package_json_parser;
mod registry;
mod types;

use deprecated::{deprecated, deprecated_prod};
use old::old;
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

            let old_pkgs = old(since.parse::<u32>().unwrap_or(4)).await?;

            if *production_pkgs_only {
                output_old(&old_pkgs.0, None);

                let num_old_pkgs = old_pkgs.0.len();

                println!("Total: {num_old_pkgs} old production dependencies");
            } else {
                output_old(&old_pkgs.0, Some("Production Dependencies:"));
                output_old(&old_pkgs.1, Some("Development Dependencies:"));

                let num_old_prods = old_pkgs.0.len();

                let num_old_devs = old_pkgs.1.len();

                println!(
                    "Total: {num_old_prods} old dependencies, {num_old_devs} old dev dependencies"
                );
            }
        }
        Some(("deprecated", sub_matches)) => {
            let production_pkgs_only = sub_matches
                .get_one::<bool>("production")
                .expect("defaulted in clap");

            if *production_pkgs_only {
                let deprecated = deprecated_prod().await?;

                output_deprecated(&deprecated, None);

                let num_depr_pkgs = deprecated.len();

                println!("Total: {num_depr_pkgs} deprecated production dependencies");
            } else {
                let (prod_depr, dev_depr) = deprecated().await?;

                output_deprecated(&prod_depr, Some("Production Dependencies:"));
                output_deprecated(&dev_depr, Some("Development Dependencies:"));

                let num_depr_prods = prod_depr.len();

                let num_depr_devs = dev_depr.len();

                println!(
                    "Total: {num_depr_prods} deprecated dependencies, {num_depr_devs} deprecated dev dependencies"
                );
            };
        }
        _ => unreachable!(),
    }

    Ok(())
}

// Utils

fn output_old(old_pkgs: &Vec<OldPkgDetails>, tag_line: Option<&str>) {
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
    } in old_pkgs
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
    if tag_line.is_some() {
        println!("{}", tag_line.unwrap());
        println!();
    }

    for VersionObjectWithDeprecated { name, version, .. } in pkgs {
        println!("{name}@{version}");
    }
    println!();
}
