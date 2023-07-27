//! Module defining utilities for detecting deprecated local packages.
//!
//! The entry point to this module is [deprecated].
//!
//! We can determine deprecation for a package version with
//! `GET https://registry.npmjs.org/:package/:version` (see [npm registry] for details).
//! Note that we pass the package name along its version, which we grab from the current workspace's package.json.
//!
//! The resulting JSON version object comes with the `deprecated` field, which is in the
//! following formats:
//!
//! - string: a deprecation message usually specifying which version to update to, but useless for our means
//! - boolean: which directly tells us if the package is deprecated or not
//! - undefined: this is the normal case for non-deprecated packages
//!
//! [npm registry]: https://github.com/npm/registry/blob/master/docs/responses/package-metadata.md

use std::error::Error;
use std::path::Path;

use futures::future;

use crate::package_json::get_deps_version;
use crate::registry::{pkg_version_info, DeprecatedField, VersionObject};
use crate::types::PkgNameAndVersion;

/// Takes workspace paths and returns a string describing deprecated packages from these workspaces.
///
/// ## Parameters
///
/// | Parameter | Description |
/// | --------- | ----------- |
/// | **prod_pkgs_only:** | Ignore development dependencies. |
/// | **path:**           | Path to the root package.json containing workspace names. |
/// | **workspaces:**     | Workspaces to check installed dependencies and versions from. |
pub async fn get_deprecated_packages(
    path: &Path,
    workspaces: &[String],
    include_development_packages: bool,
) -> String {
    let deps_by_workspace: Vec<(Vec<PkgNameAndVersion>, Vec<PkgNameAndVersion>)> = workspaces
        .iter()
        .flat_map(|workspace| {
            get_deps_version(&path.join(workspace), path, workspace == "frontend")
        })
        .collect();

    // Wait for all deps to be tested for deprecation, and zip them together with their workspace name
    let deprecated_deps_by_workspace = future::join_all(
        deps_by_workspace
            .into_iter()
            .map(|(prod, dev)| filter_deprecated_packages((prod, dev))),
    )
    .await
    .into_iter()
    // Since errors were already "flattened" inside of `get_deprecated_deps` we can safely unwrap here
    .map(|version_objs| version_objs.unwrap())
    .zip(workspaces);

    deprecated_deps_by_workspace
        .into_iter()
        .fold(String::new(), |acc, ((prod, dev), workspace)| {
            let output = get_output((&prod, &dev), include_development_packages);
            format!("{acc}\n[{workspace}] deprecated packages:\n{output}")
        })
}

/// Return a tuple containing deprecated production & development packages.
///
/// Fails if the filter operation fails for either production or development deps.
async fn filter_deprecated_packages(
    (prod_deps, dev_deps): (Vec<PkgNameAndVersion>, Vec<PkgNameAndVersion>),
) -> Result<(Vec<VersionObject>, Vec<VersionObject>), Box<dyn Error>> {
    Ok((
        filter_deprecated(&prod_deps).await?,
        filter_deprecated(&dev_deps).await?,
    ))
}

/// Filters deprecated packages. Ignores errors that may occur while fetching version info.
///
/// Assessing whether a package version is deprecated requires reading the `deprecated` field from the package info:
///
/// - if the field isn't there, the package is not deprecated
/// - if the field is exists:
///     - and is a boolean, `true` would mean the package is deprecated
///     - and is a string, the package is deprecated
async fn filter_deprecated(
    deps: &[PkgNameAndVersion],
) -> Result<Vec<VersionObject>, Box<dyn Error>> {
    let abbr_version_objects: Vec<Result<VersionObject, Box<dyn Error>>> =
        future::join_all(deps.iter().map(|PkgNameAndVersion(pkg_name, version)| {
            pkg_version_info(pkg_name.as_str(), version.as_str())
        }))
        .await;

    Ok(abbr_version_objects
        .into_iter()
        .flatten()
        .filter(|vo| match &vo.deprecated {
            Some(depr_field) => match depr_field {
                DeprecatedField::String(_) => true,
                DeprecatedField::Bool(b) => *b,
            },
            None => false,
        })
        .collect())
}

/// Returns the entire output for the deprecated task, including workspace headers
/// and statistics.
fn get_output(
    (prod_deprecated, dev_deprecated): (&[VersionObject], &[VersionObject]),
    include_development_packages: bool,
) -> String {
    if include_development_packages {
        let res_prod = get_pkgs_output(prod_deprecated, Some("production:"));
        let res_dev = get_pkgs_output(dev_deprecated, Some("development:"));

        let num_depr_prods = prod_deprecated.len();
        let num_depr_devs = dev_deprecated.len();

        format!(
        "{res_prod}{res_dev}\n  total: {num_depr_prods} deprecated dependenc{end_1}, {num_depr_devs} deprecated dev dependenc{end_2}\n",
        end_1 = if num_depr_prods == 1 { "y" } else { "ies" },
        end_2 = if num_depr_devs == 1 { "y" } else { "ies" },
      )
    } else {
        let res = get_pkgs_output(prod_deprecated, None);

        let num_depr_pkgs = prod_deprecated.len();

        format!("{res}\n  total: {num_depr_pkgs} deprecated production dependencies\n")
    }
}

/// Returns output for packages only, without headers or statistics.
fn get_pkgs_output(pkgs: &[VersionObject], tag_line: Option<&str>) -> String {
    let approx_len_name = 10;
    let approx_len_version = 8;

    // Perf trick: allocating capacity in advance to avoid reallocating a bunch of times as the string grows
    let mut res = String::with_capacity(pkgs.len() * 2 * (approx_len_name + approx_len_version));

    if pkgs.is_empty() {
        return res;
    }

    if let Some(tag) = tag_line {
        res.push_str(format!("\n  {tag}\n").as_str());
    }

    let extra_space = if tag_line.is_some() {
        " ".repeat(4)
    } else {
        " ".repeat(2)
    };

    for VersionObject { name, version, .. } in pkgs {
        res.push_str(format!("\n{extra_space}{name}@{version}").as_str());
    }

    res.push('\n');
    res
}

#[cfg(test)]
mod tests {
    use once_cell::sync::Lazy;

    use super::*;

    static DEPR_PKG_DETAILS: Lazy<Vec<VersionObject>> = Lazy::new(|| {
        vec![
            VersionObject {
                name: "depr1".to_owned(),
                version: "0.0.1".to_owned(),
                deprecated: Some(DeprecatedField::String("AAAAAAAAAAAAAAAAAA".to_owned())),
            },
            VersionObject {
                name: "depr2".to_owned(),
                version: "0.0.1".to_owned(),
                deprecated: Some(DeprecatedField::Bool(true)),
            },
            VersionObject {
                name: "not_depr".to_owned(),
                version: "0.0.1".to_owned(),
                deprecated: Some(DeprecatedField::Bool(false)),
            },
        ]
    });

    #[tokio::test(flavor = "multi_thread")]
    async fn get_deprecated_packages_test() -> Result<(), Box<dyn Error>> {
        let path = Path::new("./test-assets/monorepo/");

        let workspaces = vec![
            "backend/".to_owned(),
            "common/".to_owned(),
            "frontend/".to_owned(),
        ];

        let output = get_deprecated_packages(path, &workspaces, true).await;

        assert!(output.contains("backend/"));
        assert!(output.contains("common/"));
        assert!(output.contains("frontend/"));
        assert!(output.contains("@babel/polyfill@7.12.1"));
        assert!(output.contains("noty@3.2.0-beta-deprecated"));

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn should_get_deprecated_deps() -> Result<(), Box<dyn Error>> {
        let prod = vec![PkgNameAndVersion(
            "@babel/polyfill".to_owned(),
            "7.12.1".to_owned(),
        )];
        let devs = vec![PkgNameAndVersion(
            "file-loader".to_owned(),
            "1.1.11".to_owned(),
        )];
        let maybe_deprecated = filter_deprecated_packages((prod, devs)).await;

        assert!(maybe_deprecated.is_ok());

        let (prod_depr, dev_depr) = maybe_deprecated?;

        assert!(prod_depr.iter().any(
            |VersionObject {
                 name,
                 version,
                 deprecated,
             }| name == "@babel/polyfill"
                && version == "7.12.1"
                && deprecated.is_some()
        ));

        assert!(dev_depr.is_empty());

        Ok(())
    }

    #[test]
    fn get_deprecated_output_test() {
        let (in1, in2) = (DEPR_PKG_DETAILS.to_vec(), DEPR_PKG_DETAILS.to_vec());

        let output = get_output((&in1, &in2), true);

        assert_eq!(output, "\n  production:\n\n    depr1@0.0.1\n    depr2@0.0.1\n    not_depr@0.0.1\n\n  development:\n\n    depr1@0.0.1\n    depr2@0.0.1\n    not_depr@0.0.1\n\n  total: 3 deprecated dependencies, 3 deprecated dev dependencies\n");

        let output = get_output((&in1, &in2), false);

        assert_eq!(output, "\n  depr1@0.0.1\n  depr2@0.0.1\n  not_depr@0.0.1\n\n  total: 3 deprecated production dependencies\n");
    }

    #[test]
    fn format_deprecated_output_test() {
        let output = get_pkgs_output(DEPR_PKG_DETAILS.as_slice(), None);

        assert_eq!(output, "\n  depr1@0.0.1\n  depr2@0.0.1\n  not_depr@0.0.1\n");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn should_filter_deprecated_pkgs() -> Result<(), Box<dyn Error>> {
        let deps = vec![PkgNameAndVersion("core-js".to_owned(), "3.19.0".to_owned())];

        let res = filter_deprecated(&deps).await?;

        let expected = VersionObject {
            name: "core-js".to_owned(),
            version: "3.19.0".to_owned(),
            deprecated: Some(DeprecatedField::String("core-js@<3.23.3 is no longer maintained and not recommended for usage due to the number of issues. Because of the V8 engine whims, feature detection in old core-js versions could cause a slowdown up to 100x even if nothing is polyfilled. Some versions have web compatibility issues. Please, upgrade your dependencies to the actual version of core-js.".to_owned())),
        };

        assert_eq!(vec![expected], res);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn should_filter_out_fake_pkg() -> Result<(), Box<dyn Error>> {
        let deps = vec![PkgNameAndVersion("fake-js".to_owned(), "3.19.0".to_owned())];

        let res = filter_deprecated(&deps).await;

        assert!(res.is_ok());

        assert!(res.unwrap().is_empty());

        Ok(())
    }
}
