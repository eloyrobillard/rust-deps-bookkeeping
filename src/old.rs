use std::error::Error;

use chrono::{DateTime, Utc, FixedOffset};
use futures::future;
use once_cell::sync::Lazy;

use crate::registry::PackageMetadata;
use crate::types::{PkgName, PkgNameAndVersion, Version, OldPkgDetails};

use super::package_json_parser::get_deps_version;
use super::registry::pkg_info;

// Use same date for all
static DATE: Lazy<DateTime<Utc>> = Lazy::new(Utc::now);

#[derive(Debug, PartialEq, Eq)]
pub struct PkgAgeDetails(PkgName, Version, DateTime<FixedOffset>, u32, PackageMetadata);

pub async fn old(max_old: u32) -> Result<(Vec<OldPkgDetails>, Vec<OldPkgDetails>), Box<dyn Error>> {
    let (deps_versions, dev_deps_versions) = get_deps_version()?;

    Ok((
        get_old_pkgs(max_old, deps_versions).await?,
        get_old_pkgs(max_old, dev_deps_versions).await?,
    ))
}

async fn get_old_pkgs(
    max_old: u32,
    deps: Vec<PkgNameAndVersion>,
) -> Result<Vec<OldPkgDetails>, Box<dyn Error>> {
    // pkg info -> has `time` per version
    let maybe_pkgs_data = future::join_all(deps.into_iter().map(to_pkg_updated_tuple)).await;

    // NOTE vec_of_res.into_iter().flatten() filters Ok values
    let pkgs_data = maybe_pkgs_data.into_iter().flatten().collect();

    Ok(filter_old(max_old, pkgs_data))
}

fn filter_old(max_age: u32, data: Vec<PkgAgeDetails>) -> Vec<OldPkgDetails> {
    data.into_iter()
        .filter_map(
            |PkgAgeDetails(name, version, date_version, age_version, pkg_metadata)| {
                match age_version > max_age {
                    true => {
                        // return latest_version date+age
                        let last_update_str = pkg_metadata.time.get("modified").unwrap();
                        let date_latest_version =
                            DateTime::parse_from_rfc3339(last_update_str.as_str()).unwrap();

                        let age_latest_version = DATE.years_since(date_latest_version.into()).unwrap();

                        Some(OldPkgDetails {
                            name,
                            version,
                            date_version,
                            age_version,
                            latest_version: pkg_metadata.dist_tags.get("latest").unwrap().to_string(),
                            date_latest_version,
                            age_latest_version,
                        })
                    }
                    false => None,
                }
            },
        )
        .collect()
}

async fn to_pkg_updated_tuple(
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn to_pkg_updated_tuple_test() -> Result<(), Box<dyn Error>> {
        let PkgAgeDetails(pkg, _, last_update, ..) =
            to_pkg_updated_tuple(PkgNameAndVersion("react".to_owned(), "18.2.0".to_owned()))
                .await?;

        assert_eq!(pkg, "react");

        assert_eq!(last_update.to_string(), "2022-06-14 19:46:38.369 +00:00");

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn old_test() -> Result<(), Box<dyn Error>> {
        let maybe_old = old(4).await;

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
             }| name == "file-loader" && date_version.to_string() == "2018-03-01 22:55:18.724 +00:00"
        ));

        Ok(())
    }

    #[test]
    fn filter_old_test() {
        let max_age = 4;

        let data = vec![
            PkgAgeDetails(
                "ok".to_owned(),
                "0.0.1".to_owned(),
                FixedOffset::west_opt(5 * 3600).unwrap().with_ymd_and_hms(2022, 1, 1, 0, 0, 0).unwrap(),
                1,
                DEFAULT_METADATA.clone(),
            ),
            PkgAgeDetails(
                "old1".to_owned(),
                "0.0.1".to_owned(),
                FixedOffset::west_opt(5 * 3600).unwrap().with_ymd_and_hms(2000, 1, 1, 0, 0, 0).unwrap(),
                23,
                DEFAULT_METADATA.clone(),
            ),
            PkgAgeDetails(
                "old2".to_owned(),
                "0.0.1".to_owned(),
                FixedOffset::west_opt(5 * 3600).unwrap().with_ymd_and_hms(2016, 1, 1, 0, 0, 0).unwrap(),
                7,
                DEFAULT_METADATA.clone(),
            ),
        ];

        let old_data = filter_old(max_age, data);

        assert_eq!(
            old_data,
            vec![
                OldPkgDetails {
                    name: "old1".to_owned(),
                    version: "0.0.1".to_owned(),
                    date_version: FixedOffset::west_opt(5 * 3600).unwrap().with_ymd_and_hms(2000, 1, 1, 0, 0, 0).unwrap(),
                    age_version: 23,
                    latest_version: "0.0.1".to_owned(),
                    date_latest_version: FixedOffset::west_opt(0).unwrap().with_ymd_and_hms(2023, 6, 14, 19, 46, 38).unwrap(),
                    age_latest_version: 0,
                },
                OldPkgDetails {
                    name: "old2".to_owned(),
                    version: "0.0.1".to_owned(),
                    date_version: FixedOffset::west_opt(5 * 3600).unwrap().with_ymd_and_hms(2016, 1, 1, 0, 0, 0).unwrap(),
                    age_version: 7,
                    latest_version: "0.0.1".to_owned(),
                    date_latest_version: FixedOffset::west_opt(0).unwrap().with_ymd_and_hms(2023, 6, 14, 19, 46, 38).unwrap(),
                    age_latest_version: 0,
                },
            ]
        );
    }
}
