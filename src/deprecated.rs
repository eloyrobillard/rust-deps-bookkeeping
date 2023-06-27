use std::error::Error;

use futures::future;

use crate::package_json_parser::{get_deps_version_from_deps_field, get_deps_version_from_pkgs_field};
use crate::registry::{pkg_version_info, DeprecatedField, VersionObjectWithDeprecated};
use crate::types::PkgNameAndVersion;

pub async fn deprecated(
    path_pkg_json: Option<&str>,
    path_lock_json: Option<&str>,
) -> Result<
    (
        Vec<VersionObjectWithDeprecated>,
        Vec<VersionObjectWithDeprecated>,
    ),
    Box<dyn Error>,
> {
    let (deps_versions, dev_deps_versions) = get_deps_version_from_deps_field(path_pkg_json, path_lock_json)?;

    Ok((
        get_deprecated_pkgs(&deps_versions).await?,
        get_deprecated_pkgs(&dev_deps_versions).await?,
    ))
}

pub async fn deprecated_monorepo(
    path_pkg_json: Option<&str>,
    path_lock_json: Option<&str>,
    in_frontend: bool
) -> Result<
    (
        Vec<VersionObjectWithDeprecated>,
        Vec<VersionObjectWithDeprecated>,
    ),
    Box<dyn Error>,
> {
    let (deps_versions, dev_deps_versions) = get_deps_version_from_pkgs_field(path_pkg_json, path_lock_json, in_frontend)?;

    Ok((
        get_deprecated_pkgs(&deps_versions).await?,
        get_deprecated_pkgs(&dev_deps_versions).await?,
    ))
}

async fn get_deprecated_pkgs(
    deps: &[PkgNameAndVersion],
) -> Result<Vec<VersionObjectWithDeprecated>, Box<dyn Error>> {
    let abbr_version_objects: Vec<Result<VersionObjectWithDeprecated, Box<dyn Error>>> =
        future::join_all(deps.iter().map(|PkgNameAndVersion(pkg_name, version)| {
            pkg_version_info(pkg_name.as_str(), version.as_str())
        }))
        .await;

    Ok(abbr_version_objects
        .into_iter()
        .flatten()
        .filter(|vo| match vo.deprecated.is_some() {
            true => match vo.deprecated.as_ref().unwrap() {
                DeprecatedField::String(_) => true,
                DeprecatedField::Bool(b) => *b,
            },
            false => false,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_JSON_PATH: &str = "test-assets/";

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn deprecated_test() -> Result<(), Box<dyn Error>> {
        let deprecated = deprecated(Some(TEST_JSON_PATH), Some(TEST_JSON_PATH)).await;

        assert!(deprecated.is_ok());

        let (depr_deps, depr_dev_deps) = deprecated?;

        assert!(depr_dev_deps
            .iter()
            .any(|obj| obj.name == "core-js" && obj.version == "3.19.0"));

        assert!(depr_deps
            .iter()
            .any(|obj| obj.name == "uuid" && obj.version == "3.4.0"));

        assert!(depr_deps.iter().all(|obj| obj.deprecated.is_some()));

        Ok(())
    }
}
