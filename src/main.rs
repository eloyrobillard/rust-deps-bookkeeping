use clap::{arg, Command};

mod deprecated;
mod get_pkg_info;
mod old;
mod registry;
mod types;

use deprecated::deprecated;
use get_pkg_info::parse_package_json;
use old::old;

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
                .arg(arg!(--path <PATH> "Specify the path to the root package.json").default_value(""))
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
        Some(("old", sub_matches)) => {
            let since = sub_matches
                .get_one::<String>("since")
                .expect("defaulted in clap");

            let since = since.parse::<u32>().unwrap_or(4);

            let production_pkgs_only = sub_matches
                .get_one::<bool>("production")
                .expect("defaulted in clap");

            let path = sub_matches
                .get_one::<String>("path")
                .expect("defaulted in clap");

            let pkg_json = parse_package_json(path)?;

            match pkg_json.workspaces {
                None => panic!("No workspaces found in package.json. Cannot read the path to the necessary dependency information."),
                Some(workspaces) => println!("{}", old(since, *production_pkgs_only, path, &workspaces).await),
            }
        }
        Some(("deprecated", sub_matches)) => {
            let production_pkgs_only = sub_matches
                .get_one::<bool>("production")
                .expect("defaulted in clap");

            let path = sub_matches
                .get_one::<String>("path")
                .expect("defaulted in clap");

            let pkg_json = parse_package_json(path)?;

            match pkg_json.workspaces {
                None => panic!("No workspaces found in package.json. Cannot read the path to the necessary dependency information."),
                Some(workspaces) => println!("{}", deprecated(*production_pkgs_only, path, &workspaces).await),
            }
        }
        _ => unreachable!(),
    }

    Ok(())
}
