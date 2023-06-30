use std::collections::HashMap;
use std::error::Error;

use serde::Deserialize;

use crate::types::{PkgName, Version};

#[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
pub struct PackageMetadata {
    pub name: String,
    #[serde(rename = "dist-tags")]
    pub dist_tags: HashMap<String, String>,
    pub versions: HashMap<String, VersionObjectWithDeps>,
    pub time: HashMap<String, String>,
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
pub struct VersionObjectWithDeps {
    pub name: String,
    pub version: String,
    pub dependencies: Option<HashMap<PkgName, Version>>,
    #[serde(rename = "devDependencies")]
    pub dev_dependencies: Option<HashMap<PkgName, Version>>,
}

// LINK https://users.rust-lang.org/t/how-to-use-multiple-types-for-a-field-in-serde-json/36714/3
// NOTE allows to parse `deprecated` field to either String or bool
#[derive(Clone, Deserialize, Debug, PartialEq, Eq)]
#[serde(untagged)]
pub enum DeprecatedField {
    String(String),
    Bool(bool)
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct VersionObjectWithDeprecated {
    pub name: String,
    pub version: String,
    pub deprecated: Option<DeprecatedField>,
}

pub async fn pkg_info(pkg_name: &str) -> Result<PackageMetadata, Box<dyn Error>> {
    let resp = reqwest::get(format!("https://registry.npmjs.org/{pkg_name}")).await?;

    match serde_json::from_str(resp.text().await?.as_str()) {
        Ok(json) => Ok(json),
        Err(e) => Err(Box::<dyn Error>::from(format!("{pkg_name}: {e}"))),
    }
}

pub async fn pkg_version_info(
    pkg_name: &str,
    version: &str,
) -> Result<VersionObjectWithDeprecated, Box<dyn Error>> {
    let resp = reqwest::Client::new()
        .get(format!("https://registry.npmjs.org/{pkg_name}/{version}"))
        .send()
        .await?;

    match serde_json::from_str(resp.text().await?.as_str()) {
        Ok(json) => Ok(json),
        Err(e) => Err(Box::<dyn Error>::from(format!("{pkg_name}: {e}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn should_return_react_info() {
        let react_metadata = pkg_info("react").await;

        assert!(react_metadata.is_ok());

        let react_metadata = react_metadata.unwrap();

        assert_eq!(react_metadata.name, "react");

        assert!(react_metadata.dist_tags.contains_key("latest"));

        assert!(react_metadata.versions.contains_key("18.2.0"));

        assert!(react_metadata.time.contains_key("18.2.0"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn pkg_info_can_return_error() {
        let res = pkg_info("ReAcT").await;

        assert!(res.is_err())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn should_return_react_version_info() {
        let react = pkg_version_info("react", "0.8.0").await;

        assert!(react.is_ok());

        let react = react.unwrap();

        assert_eq!(react.version, "0.8.0");

        assert!(react.deprecated.is_none());

        let querystring = pkg_version_info("querystring", "0.2.0").await;

        assert!(querystring.is_ok());

        let querystring = querystring.unwrap();

        assert_eq!(querystring.version, "0.2.0");

        assert!(querystring.deprecated.is_some())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn should_return_error_for_fake_version() {
        let res = pkg_version_info("react", "A.8.0").await;

        assert!(res.is_err())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn should_return_error_for_fake_pkg() {
        let res = pkg_version_info("ReAcT", "0.1.0").await;

        assert!(res.is_err())
    }
}
