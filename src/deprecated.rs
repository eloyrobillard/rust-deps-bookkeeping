use std::error::Error;

use futures::future;

use crate::get_pkg_info::get_deps_version;
use crate::registry::{pkg_version_info, DeprecatedField, VersionObjectWithDeprecated};
use crate::types::PkgNameAndVersion;

pub async fn deprecated(prod_pkgs_only: bool, path: &str, workspaces: &[String]) -> String {
    let deps_by_workspace: Vec<(Vec<PkgNameAndVersion>, Vec<PkgNameAndVersion>)> = workspaces
        .iter()
        .flat_map(|workspace| {
            get_deps_version(
                format!("{path}{workspace}").as_str(),
                path,
                workspace == "frontend",
            )
        })
        .collect();

    let deprecated_deps_by_workspace = future::join_all(
        deps_by_workspace
            .into_iter()
            .map(|(prod, dev)| get_deprecated_deps((prod, dev))),
    )
    .await
    .into_iter()
    .flatten()
    .zip(workspaces);

    deprecated_deps_by_workspace
        .into_iter()
        .fold(String::new(), |acc, ((prod, dev), workspace)| {
            let output = get_deprecated_output((&prod, &dev), prod_pkgs_only);
            format!("{acc}\n[{workspace}] deprecated packages:\n{output}")
        })
}

/// Return a tuple containing deprecated production & development packages.
///
/// Fails if either filter operation fails.
async fn get_deprecated_deps(
    (prod_deps, dev_deps): (Vec<PkgNameAndVersion>, Vec<PkgNameAndVersion>),
) -> Result<
    (
        Vec<VersionObjectWithDeprecated>,
        Vec<VersionObjectWithDeprecated>,
    ),
    Box<dyn Error>,
> {
    Ok((
        filter_deprecated(&prod_deps).await?,
        filter_deprecated(&dev_deps).await?,
    ))
}

fn get_deprecated_output(
    (prod_deprecated, dev_deprecated): (
        &[VersionObjectWithDeprecated],
        &[VersionObjectWithDeprecated],
    ),
    prod_pkgs_only: bool,
) -> String {
    if prod_pkgs_only {
        let res = format_deprecated_output(prod_deprecated, None);

        let num_depr_pkgs = prod_deprecated.len();

        format!("{res}\n  total: {num_depr_pkgs} deprecated production dependencies\n")
    } else {
        let res_prod = format_deprecated_output(prod_deprecated, Some("production:"));
        let res_dev = format_deprecated_output(dev_deprecated, Some("development:"));

        let num_depr_prods = prod_deprecated.len();

        let num_depr_devs = dev_deprecated.len();

        format!(
            "{res_prod}{res_dev}\n  total: {num_depr_prods} deprecated dependenc{end_1}, {num_depr_devs} deprecated dev dependenc{end_2}\n",
            end_1 = if num_depr_prods == 1 { "y" } else { "ies" },
            end_2 = if num_depr_devs == 1 { "y" } else { "ies" },
        )
    }
}

fn format_deprecated_output(
    pkgs: &[VersionObjectWithDeprecated],
    tag_line: Option<&str>,
) -> String {
    let approx_len_name = 10;
    let approx_len_version = 8;

    // NOTE Perf trick: allocating capacity in advance to avoid reallocating a bunch of times as the String grows
    let mut res = String::with_capacity(pkgs.len() * (approx_len_name + approx_len_version));

    if pkgs.is_empty() {
        return res;
    }

    if let Some(tag) = tag_line {
        res.push_str(format!("\n  {tag}\n").as_str());
    }

    let extra_space = if tag_line.is_some() { " ".repeat(4) } else { " ".repeat(2) };

    for VersionObjectWithDeprecated { name, version, .. } in pkgs {
        res.push_str(format!("\n{extra_space}{name}@{version}").as_str());
    }

    res.push('\n');
    res
}

/// Returns deprecated packages. Errors while fetching version info are ignored.
async fn filter_deprecated(
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
    use once_cell::sync::Lazy;

    use super::*;

    static DEPR_PKG_DETAILS: Lazy<Vec<VersionObjectWithDeprecated>> = Lazy::new(|| {
        vec![
            VersionObjectWithDeprecated {
                name: "depr1".to_owned(),
                version: "0.0.1".to_owned(),
                deprecated: Some(DeprecatedField::String("AAAAAAAAAAAAAAAAAA".to_owned())),
            },
            VersionObjectWithDeprecated {
                name: "depr2".to_owned(),
                version: "0.0.1".to_owned(),
                deprecated: Some(DeprecatedField::Bool(true)),
            },
            VersionObjectWithDeprecated {
                name: "not_depr".to_owned(),
                version: "0.0.1".to_owned(),
                deprecated: Some(DeprecatedField::Bool(false)),
            },
        ]
    });

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn deprecated_test() -> Result<(), Box<dyn Error>> {
        let path = "./test-assets/monorepo/";

        let workspaces = vec![
            "backend/".to_owned(),
            "common/".to_owned(),
            "frontend/".to_owned(),
        ];

        let output = deprecated(false, path, &workspaces).await;

        assert!(output.contains("backend/"));
        assert!(output.contains("common/"));
        assert!(output.contains("frontend/"));
        assert!(output.contains("@babel/polyfill@7.12.1"));
        assert!(output.contains("noty@3.2.0-beta-deprecated"));

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn should_get_deprecated_deps() -> Result<(), Box<dyn Error>> {
        let prod = vec![PkgNameAndVersion(
            "@babel/polyfill".to_owned(),
            "7.12.1".to_owned(),
        )];
        let devs = vec![PkgNameAndVersion(
            "file-loader".to_owned(),
            "1.1.11".to_owned(),
        )];
        let maybe_deprecated = get_deprecated_deps((prod, devs)).await;

        assert!(maybe_deprecated.is_ok());

        let (prod_depr, dev_depr) = maybe_deprecated?;

        assert!(prod_depr.iter().any(
            |VersionObjectWithDeprecated {
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

        let output = get_deprecated_output((&in1, &in2), false);

        assert_eq!(output, "\n  production:\n\n    depr1@0.0.1\n    depr2@0.0.1\n    not_depr@0.0.1\n\n  development:\n\n    depr1@0.0.1\n    depr2@0.0.1\n    not_depr@0.0.1\n\n  total: 3 deprecated dependencies, 3 deprecated dev dependencies\n");

        let output = get_deprecated_output((&in1, &in2), true);

        assert_eq!(output, "\n  depr1@0.0.1\n  depr2@0.0.1\n  not_depr@0.0.1\n\n  total: 3 deprecated production dependencies\n");
    }

    #[test]
    fn format_deprecated_output_test() {
        let output = format_deprecated_output(DEPR_PKG_DETAILS.as_slice(), None);

        assert_eq!(output, "\n  depr1@0.0.1\n  depr2@0.0.1\n  not_depr@0.0.1\n");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn should_filter_deprecated_pkgs() -> Result<(), Box<dyn Error>> {
        let deps = vec![PkgNameAndVersion("core-js".to_owned(), "3.19.0".to_owned())];

        let res = filter_deprecated(&deps).await?;

        let expected = VersionObjectWithDeprecated {
            name: "core-js".to_owned(),
            version: "3.19.0".to_owned(),
            deprecated: Some(DeprecatedField::String("core-js@<3.23.3 is no longer maintained and not recommended for usage due to the number of issues. Because of the V8 engine whims, feature detection in old core-js versions could cause a slowdown up to 100x even if nothing is polyfilled. Some versions have web compatibility issues. Please, upgrade your dependencies to the actual version of core-js.".to_owned())),
        };

        assert_eq!(vec![expected], res);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn should_filter_out_fake_pkg() -> Result<(), Box<dyn Error>> {
        let deps = vec![PkgNameAndVersion("fake-js".to_owned(), "3.19.0".to_owned())];

        let res = filter_deprecated(&deps).await;

        assert!(res.is_ok());

        assert!(res.unwrap().is_empty());

        Ok(())
    }
}
