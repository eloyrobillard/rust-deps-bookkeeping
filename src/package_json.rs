use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

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
    packages: HashMap<String, PackageLockDepInfo>,
}

#[derive(Clone, Debug, Deserialize)]
struct PackageLockDepInfo {
    version: Option<String>,
}

type InstalledDeps = Vec<PkgName>;

type InstalledDevDeps = Vec<PkgName>;

pub fn get_deps_names(
    json_path: &Path,
) -> Result<(InstalledDeps, InstalledDevDeps), Box<dyn Error>> {
    let pkg_json = parse_package_json(json_path)?;

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

/// Retrieves the version of packages read from package-lock.json's `packages` field.
///
/// This is useful for mono-repo structures, where dependencies' metadata
/// is not present in the root package-lock's `dependencies` field.
pub fn get_deps_version(
    path_pkg_json: &Path,
    path_lock_json: &Path,
    in_frontend: bool,
) -> Result<(Vec<PkgNameAndVersion>, Vec<PkgNameAndVersion>), Box<dyn Error>> {
    let deps_lists: (InstalledDeps, InstalledDevDeps) = get_deps_names(path_pkg_json)?;

    let mut pkgs_info = parse_package_lock(path_lock_json)?;

    let prefix = if in_frontend {
        "frontend/node_modules/"
    } else {
        "node_modules/"
    };

    Ok((
        combine_deps_name_version(&mut pkgs_info.packages, deps_lists.0, prefix),
        combine_deps_name_version(&mut pkgs_info.packages, deps_lists.1, prefix),
    ))
}

/// Augment a list of dependency names with their version in the current project.
///
/// Takes a prefix to be able to conform to package naming in the monorepo setup
/// (`node_modules/<package name>` or `frontend/node_modules/<package name>`).
///
/// # Arguments:
///
/// * `prefix`: prefix to the package name, usually `node_modules/` or `frontend/node_modules` for the frontend dependencies.
fn combine_deps_name_version(
    deps_info: &mut HashMap<PkgName, PackageLockDepInfo>,
    deps_list: Vec<PkgName>,
    prefix: &str,
) -> Vec<PkgNameAndVersion> {
    deps_list
        .into_iter()
        .filter_map(|pkg_name| {
            deps_info
                // getting a mutable reference to the entry in order to allow the use of `mem::take`
                // `mem::take` will replace the entry with an empty field, which is okay
                // because a package cannot be both in the production and development dependencies' list
                // so we will never have to read this entry again (and we don't use this `deps_info` again)
                .get_mut(format!("{}{}", prefix, pkg_name).as_str())
                .and_then(|info| std::mem::take(&mut info.version))
                .map(|version| PkgNameAndVersion(pkg_name, version))
        })
        .collect()
}

pub fn parse_package_json(path: &Path) -> Result<PackageJson, Box<dyn Error>> {
    parse_file(path.join("package.json").as_os_str())
}

fn parse_package_lock(path: &Path) -> Result<PackageLockJson, Box<dyn Error>> {
    parse_file(path.join("package-lock.json").as_os_str())
}

fn parse_file<T: DeserializeOwned>(file_name: &std::ffi::OsStr) -> Result<T, Box<dyn Error>> {
    let file = File::open(file_name).map_err(|_| {
        Box::<dyn Error>::from(format!(
            r#"package.json not found at "{fn}""#,
            fn = file_name.to_str().unwrap()
        ))
    })?;

    let reader = BufReader::new(file);

    // using `?` to coerce serde_json::Error to Box<dyn Error>
    Ok(serde_json::from_reader(reader)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    static PROD_PKGS: [&str; 76] = [
        "react-select",
        "react-chartjs-2",
        "redux-thunk",
        "copy-to-clipboard",
        "react-helmet",
        "papaparse",
        "rc-calendar",
        "react-dropzone",
        "react-bootstrap",
        "recompose",
        "swr",
        "noty",
        "rc-tooltip",
        "react-hover-observer",
        "react-intl",
        "react-toggle",
        "react-autosize-textarea",
        "jspdf",
        "redux",
        "react-lowlight",
        "react-router",
        "react-dom",
        "draft-js",
        "react-rnd",
        "moment-timezone",
        "markdown-draft-js",
        "rc-dropdown",
        "react-draft-wysiwyg",
        "react-codemirror2",
        "react-simple-maps",
        "hunq",
        "react-refresh",
        "reselect",
        "query-string",
        "rc-progress",
        "react-tooltip",
        "react-url-query",
        "immer",
        "react-loadable",
        "lodash",
        "react-virtualized",
        "redux-mock-store",
        "debounce-promise",
        "semver",
        "react-hook-form",
        "rc-select",
        "rc-time-picker",
        "uuid",
        "prop-types",
        "redux-form",
        "react",
        "react-datepicker",
        "html2canvas",
        "rc-input-number",
        "prettysize",
        "history",
        "codemirror",
        "d3",
        "chartjs-plugin-datalabels",
        "framer-motion",
        "react-lineto",
        "react-sizes",
        "socket.io-client",
        "rc-table",
        "react-image-lightbox",
        "react-redux",
        "recharts",
        "rc-slider",
        "normalize.css",
        "chart.js",
        "highlight.js",
        "iconv-lite",
        "country-list",
        "react-router-dom",
        "moment",
        "classnames",
    ];

    static DEV_PKGS: [&str; 61] = [
        "@babel/preset-typescript",
        "@types/react-simple-maps",
        "@babel/core",
        "enzyme-to-json",
        "core-js",
        "@types/jest",
        "@testing-library/react-hooks",
        "enzyme",
        "@types/classnames",
        "@pmmmwh/react-refresh-webpack-plugin",
        "jest-environment-jsdom",
        "react-svg-loader",
        "unused-webpack-plugin",
        "webpack-cli",
        "@babel/cli",
        "@types/react",
        "postcss",
        "terser-webpack-plugin",
        "autoprefixer",
        "url-loader",
        "@babel/polyfill",
        "babel-jest",
        "css-loader",
        "style-loader",
        "optimize-css-assets-webpack-plugin",
        "raw-loader",
        "postcss-import",
        "file-loader",
        "jest-fetch-mock",
        "@types/lodash",
        "@babel/plugin-proposal-optional-chaining",
        "babel-loader",
        "html-webpack-plugin",
        "tailwindcss",
        "@babel/plugin-proposal-json-strings",
        "@babel/plugin-proposal-numeric-separator",
        "postcss-assets",
        "react-test-renderer",
        "@types/recharts",
        "clean-webpack-plugin",
        "webpack",
        "jest",
        "@babel/plugin-syntax-dynamic-import",
        "typescript",
        "@types/react-helmet",
        "babel-plugin-lodash",
        "html-loader",
        "postcss-loader",
        "worker-loader",
        "@babel/plugin-proposal-throw-expressions",
        "webpack-dev-server",
        "@types/moment-timezone",
        "cssnano",
        "html-webpack-include-assets-plugin",
        "ts-loader",
        "@babel/preset-env",
        "copy-webpack-plugin",
        "@babel/preset-react",
        "@wojtekmaj/enzyme-adapter-react-17",
        "@babel/plugin-proposal-class-properties",
        "webpack-merge",
    ];

    #[test]
    fn should_return_deps_names() {
        let list = get_deps_names(Path::new("test-assets/"));

        assert!(list.is_ok());

        let (prod, dev) = list.unwrap();

        assert!(PROD_PKGS
            .into_iter()
            .all(|name| prod.contains(&name.to_string())));
        assert!(DEV_PKGS
            .into_iter()
            .all(|name| dev.contains(&name.to_string())));
    }

    #[test]
    fn deps_list_to_version_tuple_with_prefix_test() {
        let mut deps_info = HashMap::from([(
            "a".to_owned(),
            PackageLockDepInfo {
                version: Some("1.0.0".to_owned()),
            },
        )]);

        let deps_list = vec!["a".to_owned()];

        let pkg_name_and_version = combine_deps_name_version(&mut deps_info, deps_list, "");

        assert_eq!(
            pkg_name_and_version,
            vec![PkgNameAndVersion("a".to_owned(), "1.0.0".to_owned())]
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn parse_monorepo_frontend_package_lock_test() {
        let pkg_name_and_version = parse_lock(Path::new("test-assets/monorepo/frontend/"));

        assert!(pkg_name_and_version.contains(&PkgNameAndVersion(
            "react-refresh".to_owned(),
            "0.8.3".to_owned()
        )));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn parse_monorepo_backend_package_lock_test() {
        let pkg_name_and_version = parse_lock(Path::new("test-assets/monorepo/backend/"));

        assert!(pkg_name_and_version.contains(&PkgNameAndVersion(
            "express".to_owned(),
            "4.18.2".to_owned()
        )));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn parse_monorepo_common_package_json_test() {
        let pkg_name_and_version = parse_lock(Path::new("test-assets/monorepo/common/"));

        assert!(pkg_name_and_version.contains(&PkgNameAndVersion(
            "lodash.uniq".to_owned(),
            "4.5.0".to_owned()
        )));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn parse_package_lock_test() {
        let res = parse_package_lock(Path::new("test-assets/"));

        assert!(res.is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn should_return_error_on_missing_pkg_json() {
        let res = parse_package_json(Path::new("missing-path/"));

        assert!(res.is_err());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn should_return_error_on_missing_pkg_lock() {
        let res = parse_package_lock(Path::new("missing-path/"));

        assert!(res.is_err());
    }

    // UTILS

    /// Util for easily testing the parsing of a monorepo style package-lock.json
    ///
    /// ## Arguments
    ///
    /// - **folder**: path to the chosen workspace's `package.json` (not for the root `package-lock.json`!)
    fn parse_lock(folder: &Path) -> Vec<PkgNameAndVersion> {
        let deps_lists = get_deps_names(folder);

        assert!(deps_lists.is_ok());

        let pkgs_info = parse_package_lock(Path::new("test-assets/monorepo/"));

        assert!(pkgs_info.is_ok());

        let mut pkgs = pkgs_info.unwrap();

        let prod_deps = deps_lists.unwrap().0;

        combine_deps_name_version(&mut pkgs.packages, prod_deps, "node_modules/")
    }
}
