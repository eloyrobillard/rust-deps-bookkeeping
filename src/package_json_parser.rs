use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;

use serde::{de::DeserializeOwned, Deserialize};

use crate::types::{PkgName, PkgNameAndVersion, Version};

#[derive(Clone, Debug, Deserialize)]
pub struct PackageJson {
    pub workspaces: Option<Vec<String>>,
    dependencies: Option<HashMap<PkgName, Version>>,
    #[serde(rename = "devDependencies")]
    dev_dependencies: Option<HashMap<PkgName, Version>>,
}

#[derive(Clone, Debug, Deserialize)]
struct PackageLockJson {
    dependencies: Option<HashMap<PkgName, PackageLockDepInfo>>,
    packages: HashMap<String, PackageLockDepInfo>,
}

#[derive(Clone, Debug, Deserialize)]
struct PackageLockDepInfo {
    version: Option<String>,
}

type InstalledDeps = Vec<PkgName>;

type InstalledDevDeps = Vec<PkgName>;

pub fn get_deps_lists(
    json_path: Option<&str>,
) -> Result<(InstalledDeps, InstalledDevDeps), Box<dyn Error>> {
    let pkg_json = parse_package_json(
        json_path
            .map(|path| format!("{}{}", path, "package.json"))
            .as_deref(),
    )?;

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

/// Retrieves the version of packages read from package-lock.json's `dependencies` field
///
/// This is the default for basic project structures.
///
/// On the other hand, this cannot be used for a mono-repo structure,
/// where the root package-lock.json's `dependencies` field does not contain metadata
/// for the packages used in the sub-repos.
///
/// For mono-repo structures, use [`get_deps_version_from_pkgs_field()`].
pub fn get_deps_version_from_deps_field(
    path_pkg_json: Option<&str>,
    path_lock_json: Option<&str>,
) -> Result<(Vec<PkgNameAndVersion>, Vec<PkgNameAndVersion>), Box<dyn Error>> {
    let deps_lists: (InstalledDeps, InstalledDevDeps) = get_deps_lists(path_pkg_json)?;

    let pkgs_info = parse_package_lock(
        path_lock_json
            .map(|path| format!("{}{}", path, "package-lock.json"))
            .as_deref(),
    )?;

    let deps = &pkgs_info.dependencies.unwrap();

    Ok((
        deps_list_to_version_tuple(deps, &deps_lists.0),
        deps_list_to_version_tuple(deps, &deps_lists.1),
    ))
}

/// Retrieves the version of packages read from package-lock.json's `packages` field
///
/// This is useful for mono-repo structures, where dependencies' metadata
/// is not present in the root package-lock's `dependencies` field.
///
/// For a normal repo structure, use [`get_deps_version_from_deps_field()`] instead.
pub fn get_deps_version_from_pkgs_field(
    path_pkg_json: Option<&str>,
    path_lock_json: Option<&str>,
    in_frontend: bool,
) -> Result<(Vec<PkgNameAndVersion>, Vec<PkgNameAndVersion>), Box<dyn Error>> {
    let deps_lists: (InstalledDeps, InstalledDevDeps) = get_deps_lists(path_pkg_json)?;

    let pkgs_info = parse_package_lock(
        path_lock_json
            .map(|path| format!("{}{}", path, "package-lock.json"))
            .as_deref(),
    )?;

    let prefix = if in_frontend {
        "frontend/node_modules/"
    } else {
        "node_modules/"
    };

    Ok((
        deps_list_to_version_tuple_with_prefix(&pkgs_info.packages, &deps_lists.0, prefix),
        deps_list_to_version_tuple_with_prefix(&pkgs_info.packages, &deps_lists.1, prefix),
    ))
}

fn deps_list_to_version_tuple_with_prefix(
    deps_info: &HashMap<PkgName, PackageLockDepInfo>,
    deps_list: &[PkgName],
    prefix: &str,
) -> Vec<PkgNameAndVersion> {
    deps_list
        .iter()
        .filter_map(|pkg_name| {
            deps_info
                .get(format!("{}{}", prefix, pkg_name).as_str())
                .and_then(|info| info.version.clone())
                .map_or_else(
                    || {
                        Some(PkgNameAndVersion(
                            pkg_name.clone(),
                            format!("{}{}", prefix, pkg_name),
                        ))
                    },
                    |version| Some(PkgNameAndVersion(pkg_name.clone(), version)),
                )
        })
        .collect()
}

fn deps_list_to_version_tuple(
    deps_info: &HashMap<PkgName, PackageLockDepInfo>,
    deps_list: &[PkgName],
) -> Vec<PkgNameAndVersion> {
    deps_list
        .iter()
        .filter_map(|pkg_name| {
            deps_info
                .get(pkg_name.as_str())
                .and_then(|info| info.version.clone())
                .map_or_else(
                    || Some(PkgNameAndVersion(pkg_name.clone(), pkg_name.clone())),
                    |version| Some(PkgNameAndVersion(pkg_name.clone(), version)),
                )
        })
        .collect()
}

pub fn parse_package_json(path: Option<&str>) -> Result<PackageJson, Box<dyn Error>> {
    parse_file(path.unwrap_or("package.json"))
}

fn parse_package_lock(path: Option<&str>) -> Result<PackageLockJson, Box<dyn Error>> {
    parse_file(path.unwrap_or("package-lock.json"))
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
                version: Some("1.0.0".to_owned()),
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
    async fn parse_monorepo_frontend_package_lock_test() {
        let pkg_name_and_version = parse_lock("test-assets/monorepo/frontend/");

        assert!(pkg_name_and_version.contains(&PkgNameAndVersion(
            "react-refresh".to_owned(),
            "0.8.3".to_owned()
        )));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn parse_monorepo_backend_package_lock_test() {
        let pkg_name_and_version = parse_lock("test-assets/monorepo/backend/");

        assert!(pkg_name_and_version.contains(&PkgNameAndVersion(
            "express".to_owned(),
            "4.18.2".to_owned()
        )));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn parse_monorepo_common_package_lock_test() {
        let pkg_name_and_version = parse_lock("test-assets/monorepo/common/");

        assert!(pkg_name_and_version.contains(&PkgNameAndVersion(
            "lodash.uniq".to_owned(),
            "4.5.0".to_owned()
        )));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn parse_package_lock_test() {
        let res = parse_package_lock(Some("test-assets/package-lock.json"));

        assert!(res.is_ok());
    }

    // UTILS

    fn parse_lock(folder: &str) -> Vec<PkgNameAndVersion> {
        let deps_lists = get_deps_lists(Some(folder));

        assert!(deps_lists.is_ok());

        let pkgs_info = parse_package_lock(Some("test-assets/monorepo/package-lock.json"));

        assert!(pkgs_info.is_ok());

        let pkgs = pkgs_info.unwrap();

        let prod_deps = deps_lists.unwrap().0;

        deps_list_to_version_tuple_with_prefix(&pkgs.packages, &prod_deps, "node_modules/")
    }
}
