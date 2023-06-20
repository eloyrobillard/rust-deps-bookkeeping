use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;

use serde::{de::DeserializeOwned, Deserialize};

use crate::types::{PkgName, PkgNameAndVersion, Version};

#[derive(Clone, Debug, Deserialize)]
pub struct PackageJson {
    dependencies: Option<HashMap<PkgName, Version>>,
    #[serde(rename = "devDependencies")]
    dev_dependencies: Option<HashMap<PkgName, Version>>,
}

#[derive(Debug, Deserialize)]
struct PackageLockJson {
    dependencies: HashMap<PkgName, PackageLockDepInfo>,
}

#[derive(Debug, Deserialize)]
struct PackageLockDepInfo {
    version: String,
}

type InstalledDeps = Vec<PkgName>;

type InstalledDevDeps = Vec<PkgName>;

pub fn get_deps_lists() -> Result<(InstalledDeps, InstalledDevDeps), Box<dyn Error>> {
    let pkg_json = parse_package_json()?;

    Ok((
        pkg_json
            .dependencies
            .unwrap_or(HashMap::new())
            .into_keys()
            .collect(),
        pkg_json
            .dev_dependencies
            .unwrap_or(HashMap::new())
            .into_keys()
            .collect(),
    ))
}

pub fn get_deps_version() -> Result<(Vec<PkgNameAndVersion>, Vec<PkgNameAndVersion>), Box<dyn Error>>
{
    let deps_lists: (InstalledDeps, InstalledDevDeps) = get_deps_lists()?;

    let pkgs_info = parse_package_lock()?;

    Ok((
        deps_list_to_version_tuple(&pkgs_info.dependencies, &deps_lists.0),
        deps_list_to_version_tuple(&pkgs_info.dependencies, &deps_lists.1),
    ))
}

fn deps_list_to_version_tuple(
    deps_info: &HashMap<PkgName, PackageLockDepInfo>,
    deps_list: &[PkgName],
) -> Vec<PkgNameAndVersion> {
    deps_list
        .iter()
        .map(|pkg_name| {
            PkgNameAndVersion(
                pkg_name.clone(),
                deps_info.get(pkg_name.as_str()).unwrap().version.clone(),
            )
        })
        .collect()
}

fn parse_package_json() -> Result<PackageJson, Box<dyn Error>> {
    parse_file("~package.json")
}

fn parse_package_lock() -> Result<PackageLockJson, Box<dyn Error>> {
    parse_file("~package-lock.json")
}

fn parse_file<T: DeserializeOwned>(file_name: &str) -> Result<T, Box<dyn Error>> {
    let file = File::open(file_name)?;
    let reader = BufReader::new(file);

    // using `?` to coerce serde::Error to Box<dyn Error>
    Ok(serde_json::from_reader(reader)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deps_list_to_version_tuple_test() {
        let deps_info = HashMap::from([(
            "a".to_owned(),
            PackageLockDepInfo {
                version: "1.0.0".to_owned(),
            },
        )]);

        let deps_list = vec!["a".to_owned()];

        let pkg_name_and_version = deps_list_to_version_tuple(&deps_info, &deps_list);

        assert_eq!(
            pkg_name_and_version,
            vec![PkgNameAndVersion("a".to_owned(), "1.0.0".to_owned())]
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn parse_package_lock_test() {
        let res = parse_package_lock();

        assert!(res.is_ok());
    }
}
