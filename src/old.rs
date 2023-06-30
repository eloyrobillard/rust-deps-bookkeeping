use std::error::Error;

use chrono::{DateTime, FixedOffset, Utc};
use futures::future;
use once_cell::sync::Lazy;

use crate::get_pkg_info::get_deps_version;
use crate::registry::PackageMetadata;
use crate::types::{PkgName, PkgNameAndVersion, Version};

use super::registry::pkg_info;

// Use same date for all
static DATE: Lazy<DateTime<Utc>> = Lazy::new(Utc::now);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PkgAgeDetails(
    PkgName,
    Version,
    DateTime<FixedOffset>,
    u32,
    PackageMetadata,
);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OldPkgDetails {
    pub name: PkgName,
    pub version: Version,
    pub date_version: DateTime<FixedOffset>,
    pub age_version: u32,
    pub latest_version: Version,
    pub date_latest_version: DateTime<FixedOffset>,
    pub age_latest_version: u32,
}

pub async fn old(
    since: u32,
    prod_pkgs_only: bool,
    path: &str,
    workspaces: &[String],
) -> String {
    let deps_by_workspace: Vec<(Vec<PkgNameAndVersion>, Vec<PkgNameAndVersion>)> = workspaces
        .iter()
        .flat_map(|workspace| {
            get_deps_version(
                format!("{path}{workspace}").as_str(),
                path,
                workspace == "frontend/",
            )
        })
        .collect();

    let old_deps_by_workspace = future::join_all(
        deps_by_workspace
            .into_iter()
            .map(|(prod, dev)| get_old_deps((prod, dev), since)),
    )
    .await
    .into_iter()
    .flatten()
    .zip(workspaces);

    old_deps_by_workspace
        .into_iter()
        .fold(String::new(), |acc, ((prod, dev), workspace)| {
            let output = get_old_output((&prod, &dev), prod_pkgs_only);
            format!("{acc}\n[{workspace}] old packages:\n{output}")
        })
}

async fn get_old_deps(
    (prod_deps, dev_deps): (Vec<PkgNameAndVersion>, Vec<PkgNameAndVersion>),
    max_old: u32,
) -> Result<(Vec<OldPkgDetails>, Vec<OldPkgDetails>), Box<dyn Error>> {
    async fn worker(
        max_old: u32,
        deps: Vec<PkgNameAndVersion>,
    ) -> Result<Vec<OldPkgDetails>, Box<dyn Error>> {
        let maybe_pkgs_data = future::join_all(deps.into_iter().map(to_pkg_date_tuple)).await;

        // NOTE vec_of_res.into_iter().flatten() filters Ok values
        let pkgs_data = maybe_pkgs_data.into_iter().flatten().collect();

        Ok(filter_old(max_old, pkgs_data))
    }

    Ok((
        worker(max_old, prod_deps).await?,
        worker(max_old, dev_deps).await?,
    ))
}

fn filter_old(max_age: u32, data: Vec<PkgAgeDetails>) -> Vec<OldPkgDetails> {
    data.into_iter()
        .filter_map(
            |PkgAgeDetails(name, version, date_version, age_version, pkg_metadata)| {
                (age_version > max_age).then(|| {
                    let last_update_str = pkg_metadata.time.get("modified").unwrap();

                    let date_latest_version =
                        DateTime::parse_from_rfc3339(last_update_str.as_str()).unwrap();

                    let age_latest_version = DATE.years_since(date_latest_version.into()).unwrap();

                    OldPkgDetails {
                        name,
                        version,
                        date_version,
                        age_version,
                        latest_version: pkg_metadata.dist_tags.get("latest").unwrap().to_string(),
                        date_latest_version,
                        age_latest_version,
                    }
                })
            },
        )
        .collect()
}

fn get_old_output(
    (prod_old, dev_old): (&[OldPkgDetails], &[OldPkgDetails]),
    prod_pkgs_only: bool,
) -> String {
    if prod_pkgs_only {
        let res = format_old_output(prod_old, None);

        let num_old_pkgs = prod_old.len();

        format!("{res}\n  total: {num_old_pkgs} old production dependencies\n")
    } else {
        let res_prod = format_old_output(prod_old, Some("production:"));
        let res_dev = format_old_output(dev_old, Some("development:"));

        let num_old_prods = prod_old.len();

        let num_old_devs = dev_old.len();

        format!(
            "{res_prod}{res_dev}\n  total: {num_old_prods} old dependenc{end_1}, {num_old_devs} old dev dependenc{end_2}\n",
            end_1 = if num_old_prods == 1 { "y" } else { "ies" },
            end_2 = if num_old_devs == 1 { "y" } else { "ies" },
        )
    }
}

/// ## Arguments
///
/// - **pkgs**:     packages to be formatted for print-out.
/// - **tag_line**: optional string announcing whether the printed-out packages belong to production or development.
fn format_old_output(pkgs: &[OldPkgDetails], tag_line: Option<&str>) -> String {
    let len_simple_date = 10;
    let approx_len_name = 10;
    let approx_len_version = 8;

    let extra_len_fst = 5;
    let extra_len_snd = 36;
    let extra_len_trd = 21;

    // NOTE Perf trick: allocating capacity in advance to avoid reallocating a bunch of times as the String grows
    let mut res = String::with_capacity(
        pkgs.len()
            * (len_simple_date * 2
                + approx_len_name
                + approx_len_version * 3
                + extra_len_fst
                + extra_len_snd
                + extra_len_trd),
    );

    if pkgs.is_empty() {
        return res;
    }

    if let Some(tag) = tag_line {
        res.push_str(format!("\n  {tag}\n").as_str());
    }

    let extra_space = if tag_line.is_some() { " ".repeat(4) } else { " ".repeat(2) };

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

        res.push_str(format!("\n{extra_space}{name}@{version} ({simple_date_version})\n").as_str());

        let age_diff = age_version - age_latest_version;

        let simple_date_latest_version = date_latest_version.format("%d/%m/%Y");

        res.push_str(
            format!("{extra_space}    -> {age_version} years old, {age_diff} older than latest\n").as_str(),
        );
        res.push_str(
            format!("{extra_space}        -> latest @{latest_version} ({simple_date_latest_version})\n")
                .as_str(),
        );
    }

    res
}

async fn to_pkg_date_tuple(
    PkgNameAndVersion(pkg, version): PkgNameAndVersion,
) -> Result<PkgAgeDetails, Box<dyn Error>> {
    let pkg_meta = pkg_info(pkg.as_str()).await?;

    match pkg_meta.time.get(version.as_str()) {
        Some(version_date) => {
            let version_update = DateTime::parse_from_rfc3339(version_date.as_str()).unwrap();

            let age = DATE.years_since(version_update.into()).unwrap();

            Ok(PkgAgeDetails(pkg, version, version_update, age, pkg_meta))
        }
        None => Err(Box::<dyn Error>::from(format!(
            "Version {version} of {pkg} not found in timetable"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::TimeZone;

    use super::*;

    static DEFAULT_METADATA: Lazy<PackageMetadata> = Lazy::new(|| PackageMetadata {
        name: "".to_owned(),
        dist_tags: HashMap::from([("latest".to_owned(), "0.0.1".to_owned())]),
        versions: HashMap::new(),
        time: HashMap::from([("modified".to_owned(), "2023-06-14T19:46:38Z".to_owned())]),
    });

    static VEC_PKG_AGE: Lazy<Vec<PkgAgeDetails>> = Lazy::new(|| {
        vec![
            PkgAgeDetails(
                "ok".to_owned(),
                "0.0.1".to_owned(),
                FixedOffset::west_opt(5 * 3600)
                    .unwrap()
                    .with_ymd_and_hms(2022, 1, 1, 0, 0, 0)
                    .unwrap(),
                1,
                DEFAULT_METADATA.clone(),
            ),
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
                version: "0.0.1".to_owned(),
                date_version: FixedOffset::west_opt(5 * 3600)
                    .unwrap()
                    .with_ymd_and_hms(2000, 1, 1, 0, 0, 0)
                    .unwrap(),
                age_version: 23,
                latest_version: "0.0.1".to_owned(),
                date_latest_version: FixedOffset::west_opt(0)
                    .unwrap()
                    .with_ymd_and_hms(2023, 6, 14, 19, 46, 38)
                    .unwrap(),
                age_latest_version: 0,
            },
            OldPkgDetails {
                name: "old2".to_owned(),
                version: "0.0.1".to_owned(),
                date_version: FixedOffset::west_opt(5 * 3600)
                    .unwrap()
                    .with_ymd_and_hms(2016, 1, 1, 0, 0, 0)
                    .unwrap(),
                age_version: 7,
                latest_version: "0.0.1".to_owned(),
                date_latest_version: FixedOffset::west_opt(0)
                    .unwrap()
                    .with_ymd_and_hms(2023, 6, 14, 19, 46, 38)
                    .unwrap(),
                age_latest_version: 0,
            },
        ]
    });

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn old_test() -> Result<(), Box<dyn Error>> {
        let path = "./test-assets/monorepo/";

        let workspaces = vec![
            "backend/".to_owned(),
            "common/".to_owned(),
            "frontend/".to_owned(),
        ];

        let output = old(4, false, path, &workspaces).await;

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
        let maybe_old = get_old_deps((vec![pkg1], vec![pkg2]), 4).await;

        assert!(maybe_old.is_ok());

        let (old_deps, old_dev_deps) = maybe_old?;

        assert!(old_deps.iter().any(
            |OldPkgDetails {
                 name, date_version, ..
             }| name == "chartjs-plugin-datalabels"
                && date_version.to_string() == "2018-03-21 16:54:32.583 +00:00"
        ));
        assert!(old_dev_deps.iter().any(
            |OldPkgDetails {
                 name, date_version, ..
             }| name == "file-loader"
                && date_version.to_string() == "2018-03-01 22:55:18.724 +00:00"
        ));

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn should_filter_wrong_pkg_info() -> Result<(), Box<dyn Error>> {
        let pkg1 = PkgNameAndVersion("wrong-info".to_owned(), "A.3.0".to_owned());
        let maybe_old = get_old_deps((vec![pkg1], vec![]), 4).await;

        assert!(maybe_old.is_ok());

        let (prod_deps, _) = maybe_old?;

        assert!(prod_deps
            .iter()
            .all(|OldPkgDetails { name, .. }| name != "wrong-info"));

        Ok(())
    }

    #[test]
    fn should_return_output_for_old_pkgs() {
        let (in1, in2) = (OLD_PKG_DETAILS.to_vec(), OLD_PKG_DETAILS.to_vec());

        let output = get_old_output((&in1, &in2), false);

        assert_eq!(output, "\n  production:\n\n    old1@0.0.1 (01/01/2000)\n        -> 23 years old, 23 older than latest\n            -> latest @0.0.1 (14/06/2023)\n\n    old2@0.0.1 (01/01/2016)\n        -> 7 years old, 7 older than latest\n            -> latest @0.0.1 (14/06/2023)\n\n  development:\n\n    old1@0.0.1 (01/01/2000)\n        -> 23 years old, 23 older than latest\n            -> latest @0.0.1 (14/06/2023)\n\n    old2@0.0.1 (01/01/2016)\n        -> 7 years old, 7 older than latest\n            -> latest @0.0.1 (14/06/2023)\n\n  total: 2 old dependencies, 2 old dev dependencies\n");

        let output = get_old_output((&in1, &in2), true);

        assert_eq!(output, "\n  old1@0.0.1 (01/01/2000)\n      -> 23 years old, 23 older than latest\n          -> latest @0.0.1 (14/06/2023)\n\n  old2@0.0.1 (01/01/2016)\n      -> 7 years old, 7 older than latest\n          -> latest @0.0.1 (14/06/2023)\n\n  total: 2 old production dependencies\n");
    }

    #[test]
    fn should_return_formatted_output_for_old_pkgs() {
        let output = format_old_output(&OLD_PKG_DETAILS.to_vec(), None);

        assert_eq!(output, "\n  old1@0.0.1 (01/01/2000)\n      -> 23 years old, 23 older than latest\n          -> latest @0.0.1 (14/06/2023)\n\n  old2@0.0.1 (01/01/2016)\n      -> 7 years old, 7 older than latest\n          -> latest @0.0.1 (14/06/2023)\n");
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
    fn filter_old_test() {
        let max_age = 4;

        let old_data = filter_old(max_age, VEC_PKG_AGE.to_vec());

        assert_eq!(old_data, OLD_PKG_DETAILS.to_vec());
    }
}
