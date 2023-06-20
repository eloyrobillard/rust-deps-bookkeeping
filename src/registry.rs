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
#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum DeprecatedField {
    String(String),
    Bool(bool)
}

#[derive(Debug, Deserialize)]
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
    async fn pkg_info_test() {
        assert!(pkg_info("react").await.is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn pkg_version_info_test() {
        let react = pkg_version_info("react", "0.8.0").await;

        assert!(react.is_ok());

        assert!(react.unwrap().deprecated.is_none());

        let querystring = pkg_version_info("querystring", "0.2.0").await;

        assert!(querystring.is_ok());

        assert!(querystring.unwrap().deprecated.is_some())
    }
}
