//! Module defining utilities for filtering local packages older than the specified age.
//!
//! The entry point to this module is [old].
//!
//! The information for the age of a package version is found with
//! `GET https://registry.npmjs.org/:package` (see [npm registry] for details).
//!
//! The resulting JSON package metadata comes with the following field:
//!
//! ``` json
//! "time": {
//!     "1.0.0": "2015-03-24T00:12:24.039Z",
//!     "1.0.1": ...,
//!     ...
//!     "created": "2015-03-24T00:12:24.039Z",
//!     "modified": "2022-05-16T22:27:54.741Z"
//! }
//! ```
//!
//! We can therefore extract the exact date at which a given version has been published.
//! The date follows the [RFC 3339] format, with the final Z representing zero-offset in UTC (i.e. United Kingdom time).
//!
//! We grab the version from the current workspace's package.json.
//!
//! [npm registry]: https://github.com/npm/registry/blob/master/docs/responses/package-metadata.md
//! [RFC 3339]: https://www.rfc-editor.org/rfc/rfc3339#page-6

use std::error::Error;
use std::path::Path;

use chrono::{DateTime, FixedOffset, Utc};
use futures::future;
use once_cell::sync::Lazy;

use crate::package_json::get_deps_version;
use crate::registry::PackageMetadata;
use crate::types::{PkgName, PkgNameAndVersion, Version};

use super::registry::pkg_info;

/// Use the same "now" date for all
static NOW: Lazy<DateTime<Utc>> = Lazy::new(Utc::now);

/// Contains the package info necessary when filtering by age.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PkgAgeDetails(
    PkgName,
    // local version
    Version,
    // publication date of the local version
    DateTime<FixedOffset>,
    // age of the local version
    u32,
    PackageMetadata,
);

/// Format containing all the necessary version and age info later shown to the user.
///
/// This includes info about the latest version of the package, something not present in [`PkgAgeDetails`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OldPkgDetails {
    pub name: PkgName,
    pub local_version: Version,
    pub publication_local_version: DateTime<FixedOffset>,
    pub age_local_version: u32,
    pub latest_version: Version,
    pub publication_latest_version: DateTime<FixedOffset>,
    pub age_latest_version: u32,
}

/// Takes workspace paths and returns a string describing packages older than the provided age limit.
///
/// ## Parameters
///
/// | Parameter | Description |
/// | --------- | ----------- |
/// | **since:**          | Arbitrary amount of **years** since when a package version must have been published to be considered old. |
/// | **prod_pkgs_only:** | Ignore development dependencies. |
/// | **path:**           | Path to the root package.json containing workspace names. |
/// | **workspaces:**     | Workspaces to check installed dependencies and versions from. |
pub async fn old(
    since: u32,
    prod_pkgs_only: bool,
    path: &Path,
    workspaces: &[String],
    mut writer: impl std::io::Write,
) -> Result<(), std::io::Error> {
    let deps_by_workspace: Vec<(Vec<PkgNameAndVersion>, Vec<PkgNameAndVersion>)> = workspaces
        .iter()
        .flat_map(|workspace| {
            get_deps_version(
                &path.join(workspace),
                path,
                // names for the deps in the `frontend/` start with an extra prefix
                workspace == "frontend/",
            )
        })
        .collect();

    // Wait for all deps to be tested for age, and zip them together with their workspace name
    let old_deps_by_workspace = future::join_all(
        deps_by_workspace
            .into_iter()
            .map(|(prod, dev)| get_old_deps(since, (prod, dev))),
    )
    .await
    .into_iter()
    // Since errors were already "flattened" inside of `get_old_deps` we can safely unwrap here
    .map(|details| details.unwrap())
    .zip(workspaces);

    for ((prod, dev), workspace) in old_deps_by_workspace {
        writeln!(writer, "\n[{workspace}] old packages:")?;
        get_output((&prod, &dev), prod_pkgs_only, &mut writer)?;
    }

    Ok(())
}

/// Get dates for the package versions used in the project, and filter old ones.
async fn get_old_deps(
    since: u32,
    (prod_deps, dev_deps): (Vec<PkgNameAndVersion>, Vec<PkgNameAndVersion>),
) -> Result<(Vec<OldPkgDetails>, Vec<OldPkgDetails>), Box<dyn Error>> {
    async fn worker(
        since: u32,
        deps: Vec<PkgNameAndVersion>,
    ) -> Result<Vec<OldPkgDetails>, Box<dyn Error>> {
        let maybe_pkgs_data = future::join_all(deps.into_iter().map(to_pkg_date_tuple)).await;

        // vec_of_res.into_iter().flatten() filters Ok values
        // Remove errors (either missing package info, or wrong publication date formatting)
        // and filter old packages.
        let pkgs = maybe_pkgs_data
            .into_iter()
            .flatten()
            .filter(|PkgAgeDetails(_, _, _, age_version, _)| *age_version > since)
            .collect();

        Ok(add_latest_version_info(pkgs))
    }

    Ok((
        worker(since, prod_deps).await?,
        worker(since, dev_deps).await?,
    ))
}

/// Augment a package with the date at which its in-use version was published and its age.
async fn to_pkg_date_tuple(
    PkgNameAndVersion(pkg, version): PkgNameAndVersion,
) -> Result<PkgAgeDetails, Box<dyn Error>> {
    let pkg_meta = pkg_info(pkg.as_str()).await?;

    match pkg_meta.time.get(version.as_str()) {
        Some(version_date) => {
            let version_update = DateTime::parse_from_rfc3339(version_date.as_str()).unwrap();

            let age = NOW.years_since(version_update.into()).unwrap();

            Ok(PkgAgeDetails(pkg, version, version_update, age, pkg_meta))
        }
        None => Err(Box::<dyn Error>::from(format!(
            "Version {version} of {pkg} not found in timetable"
        ))),
    }
}

/// Adds the latest version, its date of publication and age to the package data.
///
/// This info is later output to the user to allow age comparison of the installed version
/// with the latest version.
fn add_latest_version_info(pkgs: Vec<PkgAgeDetails>) -> Vec<OldPkgDetails> {
    pkgs.into_iter()
        .map(
            |PkgAgeDetails(
                name,
                local_version,
                publication_local_version,
                age_local_version,
                pkg_metadata,
            )| {
                let last_update_str = pkg_metadata.time.get("modified").unwrap();

                let date_latest_version =
                    DateTime::parse_from_rfc3339(last_update_str.as_str()).unwrap();

                let age_latest_version = NOW.years_since(date_latest_version.into()).unwrap();

                OldPkgDetails {
                    name,
                    local_version,
                    publication_local_version,
                    age_local_version,
                    latest_version: pkg_metadata.dist_tags.get("latest").unwrap().to_string(),
                    publication_latest_version: date_latest_version,
                    age_latest_version,
                }
            },
        )
        .collect()
}

/// Returns the full output for the `old` task, including workspace headers and statistics.
fn get_output(
    (prod_old, dev_old): (&[OldPkgDetails], &[OldPkgDetails]),
    prod_pkgs_only: bool,
    mut writer: impl std::io::Write,
) -> Result<(), std::io::Error> {
    if prod_pkgs_only {
        get_pkgs_output(prod_old, None, &mut writer)?;

        let num_old_pkgs = prod_old.len();

        writeln!(
            writer,
            "\n  total: {num_old_pkgs} old production dependencies",
        )?;
    } else {
        get_pkgs_output(prod_old, Some("production:"), &mut writer)?;
        get_pkgs_output(dev_old, Some("development:"), &mut writer)?;

        let num_old_prods = prod_old.len();
        let num_old_devs = dev_old.len();

        writeln!(writer, "\n  total: {num_old_prods} old dependenc{end_1}, {num_old_devs} old dev dependenc{end_2}",
        end_1 = if num_old_prods == 1 { "y" } else { "ies" },
        end_2 = if num_old_devs == 1 { "y" } else { "ies" }
    )?;
    }

    Ok(())
}

/// Return the output for old packages, without headers/footers.
///
/// ## Arguments
///
/// - **pkgs**:     packages to be formatted for print-out.
/// - **tag_line**: optional string announcing whether the printed-out packages belong to production or development.
fn get_pkgs_output(
    pkgs: &[OldPkgDetails],
    header: Option<&str>,
    mut writer: impl std::io::Write,
) -> Result<(), std::io::Error> {
    if pkgs.is_empty() {
        return Ok(());
    }

    let extra_space = match header {
        Some(header) => {
            writeln!(writer, "\n  {header}")?;
            " ".repeat(4)
        }
        None => " ".repeat(2),
    };

    for OldPkgDetails {
        name,
        local_version: version,
        publication_local_version: date_version,
        age_local_version: age_version,
        latest_version,
        publication_latest_version: date_latest_version,
        age_latest_version,
        ..
    } in pkgs
    {
        let simple_date_version = date_version.format("%d/%m/%Y");

        let simple_date_latest_version = date_latest_version.format("%d/%m/%Y");

        let age_diff = age_version - age_latest_version;

        writeln!(
            writer,
            "\n{extra_space}{name}@{version} ({simple_date_version})",
        )?;

        writeln!(
            writer,
            "{extra_space}    -> {age_version} years old, {age_diff} older than latest",
        )?;

        writeln!(
            writer,
            "{extra_space}        -> latest @{latest_version} ({simple_date_latest_version})",
        )?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::TimeZone;

    // imports everything from `old` module
    use super::*;

    static DEFAULT_METADATA: Lazy<PackageMetadata> = Lazy::new(|| PackageMetadata {
        name: "".to_owned(),
        dist_tags: HashMap::from([("latest".to_owned(), "0.0.1".to_owned())]),
        time: HashMap::from([("modified".to_owned(), "2023-06-14T19:46:38Z".to_owned())]),
    });

    static VEC_PKG_AGE: Lazy<Vec<PkgAgeDetails>> = Lazy::new(|| {
        vec![
            PkgAgeDetails(
                "old1".to_owned(),
                "0.0.1".to_owned(),
                FixedOffset::west_opt(5 * 3600)
                    .unwrap()
                    .with_ymd_and_hms(2000, 1, 1, 0, 0, 0)
                    .unwrap(),
                23,
                DEFAULT_METADATA.clone(),
            ),
            PkgAgeDetails(
                "old2".to_owned(),
                "0.0.1".to_owned(),
                FixedOffset::west_opt(5 * 3600)
                    .unwrap()
                    .with_ymd_and_hms(2016, 1, 1, 0, 0, 0)
                    .unwrap(),
                7,
                DEFAULT_METADATA.clone(),
            ),
        ]
    });

    static OLD_PKG_DETAILS: Lazy<Vec<OldPkgDetails>> = Lazy::new(|| {
        vec![
            OldPkgDetails {
                name: "old1".to_owned(),
                local_version: "0.0.1".to_owned(),
                publication_local_version: FixedOffset::west_opt(5 * 3600)
                    .unwrap()
                    .with_ymd_and_hms(2000, 1, 1, 0, 0, 0)
                    .unwrap(),
                age_local_version: 23,
                latest_version: "0.0.1".to_owned(),
                publication_latest_version: FixedOffset::west_opt(0)
                    .unwrap()
                    .with_ymd_and_hms(2023, 6, 14, 19, 46, 38)
                    .unwrap(),
                age_latest_version: 0,
            },
            OldPkgDetails {
                name: "old2".to_owned(),
                local_version: "0.0.1".to_owned(),
                publication_local_version: FixedOffset::west_opt(5 * 3600)
                    .unwrap()
                    .with_ymd_and_hms(2016, 1, 1, 0, 0, 0)
                    .unwrap(),
                age_local_version: 7,
                latest_version: "0.0.1".to_owned(),
                publication_latest_version: FixedOffset::west_opt(0)
                    .unwrap()
                    .with_ymd_and_hms(2023, 6, 14, 19, 46, 38)
                    .unwrap(),
                age_latest_version: 0,
            },
        ]
    });

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn old_test() -> Result<(), Box<dyn Error>> {
        let path = Path::new("./test-assets/monorepo/");

        let workspaces = vec![
            "backend/".to_owned(),
            "common/".to_owned(),
            "frontend/".to_owned(),
        ];

        let mut chars = Vec::new();

        old(4, false, path, &workspaces, &mut chars).await?;

        let output = String::from_utf8(chars)?;

        assert!(output.contains("backend/"));
        assert!(output.contains("common/"));
        assert!(output.contains("frontend/"));
        assert!(output.contains("pluralize@7.0.0 (20/08/2017)"));
        assert!(output.contains("lodash.map@4.6.0 (13/08/2016)"));

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn should_get_old_deps() -> Result<(), Box<dyn Error>> {
        let pkg1 = PkgNameAndVersion("chartjs-plugin-datalabels".to_owned(), "0.3.0".to_owned());
        let pkg2 = PkgNameAndVersion("file-loader".to_owned(), "1.1.11".to_owned());
        let maybe_old = get_old_deps(4, (vec![pkg1], vec![pkg2])).await;

        assert!(maybe_old.is_ok());

        let (old_deps, old_dev_deps) = maybe_old?;

        assert!(old_deps.iter().any(
            |OldPkgDetails {
                 name,
                 publication_local_version,
                 ..
             }| name == "chartjs-plugin-datalabels"
                && publication_local_version.to_string() == "2018-03-21 16:54:32.583 +00:00"
        ));
        assert!(old_dev_deps.iter().any(
            |OldPkgDetails {
                 name,
                 publication_local_version,
                 ..
             }| name == "file-loader"
                && publication_local_version.to_string() == "2018-03-01 22:55:18.724 +00:00"
        ));

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn should_filter_wrong_pkg_info() -> Result<(), Box<dyn Error>> {
        let pkg1 = PkgNameAndVersion("wrong-info".to_owned(), "A.3.0".to_owned());
        let maybe_old = get_old_deps(4, (vec![pkg1], vec![])).await;

        assert!(maybe_old.is_ok());

        let (prod_deps, _) = maybe_old?;

        assert!(prod_deps
            .iter()
            .all(|OldPkgDetails { name, .. }| name != "wrong-info"));

        Ok(())
    }

    #[test]
    fn should_return_output_for_old_pkgs() -> Result<(), std::io::Error> {
        let (in1, in2) = (OLD_PKG_DETAILS.to_vec(), OLD_PKG_DETAILS.to_vec());

        let mut bytes = Vec::new();

        get_output((&in1, &in2), false, &mut bytes)?;

        let output = String::from_utf8(bytes).unwrap();

        assert_eq!(output, "\n  production:\n\n    old1@0.0.1 (01/01/2000)\n        -> 23 years old, 23 older than latest\n            -> latest @0.0.1 (14/06/2023)\n\n    old2@0.0.1 (01/01/2016)\n        -> 7 years old, 7 older than latest\n            -> latest @0.0.1 (14/06/2023)\n\n  development:\n\n    old1@0.0.1 (01/01/2000)\n        -> 23 years old, 23 older than latest\n            -> latest @0.0.1 (14/06/2023)\n\n    old2@0.0.1 (01/01/2016)\n        -> 7 years old, 7 older than latest\n            -> latest @0.0.1 (14/06/2023)\n\n  total: 2 old dependencies, 2 old dev dependencies\n");

        let mut bytes = Vec::new();

        get_output((&in1, &in2), true, &mut bytes)?;

        let output = String::from_utf8(bytes).unwrap();

        assert_eq!(output, "\n  old1@0.0.1 (01/01/2000)\n      -> 23 years old, 23 older than latest\n          -> latest @0.0.1 (14/06/2023)\n\n  old2@0.0.1 (01/01/2016)\n      -> 7 years old, 7 older than latest\n          -> latest @0.0.1 (14/06/2023)\n\n  total: 2 old production dependencies\n");

        Ok(())
    }

    #[test]
    fn should_return_formatted_output_for_old_pkgs() -> Result<(), std::io::Error> {
        let mut bytes = Vec::new();

        get_pkgs_output(&OLD_PKG_DETAILS.to_vec(), None, &mut bytes)?;

        let output = String::from_utf8(bytes).unwrap();

        assert_eq!(output, "\n  old1@0.0.1 (01/01/2000)\n      -> 23 years old, 23 older than latest\n          -> latest @0.0.1 (14/06/2023)\n\n  old2@0.0.1 (01/01/2016)\n      -> 7 years old, 7 older than latest\n          -> latest @0.0.1 (14/06/2023)\n");

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn should_get_last_update() -> Result<(), Box<dyn Error>> {
        let PkgAgeDetails(pkg, _, last_update, ..) =
            to_pkg_date_tuple(PkgNameAndVersion("react".to_owned(), "18.2.0".to_owned())).await?;

        assert_eq!(pkg, "react");

        assert_eq!(last_update.to_string(), "2022-06-14 19:46:38.369 +00:00");

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn should_fail_to_get_last_update_of_fake_pkg() -> Result<(), Box<dyn Error>> {
        let res =
            to_pkg_date_tuple(PkgNameAndVersion("ReAcT".to_owned(), "18.2.0".to_owned())).await;

        assert!(res
            .expect_err("Should be an error")
            .to_string()
            .contains("ReAcT: missing field `name`"));

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn should_fail_to_get_last_update_of_bad_version() -> Result<(), Box<dyn Error>> {
        let res =
            to_pkg_date_tuple(PkgNameAndVersion("react".to_owned(), "A.2.0".to_owned())).await;

        assert_eq!(
            res.expect_err("Should be an error").to_string(),
            "Version A.2.0 of react not found in timetable"
        );

        Ok(())
    }

    #[test]
    fn should_add_info_about_latest_version() {
        let old_data = add_latest_version_info(VEC_PKG_AGE.to_vec());

        assert_eq!(old_data, OLD_PKG_DETAILS.to_vec());
    }
}
