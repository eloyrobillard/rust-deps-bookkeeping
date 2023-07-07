use std::collections::HashMap;
use std::error::Error;

use serde::Deserialize;

/// Type corresponding to the response from a `GET https://registry.npmjs.org/:package` request to the [npm registry].
///
/// [npm registry]: https://github.com/npm/registry/blob/master/docs/responses/package-metadata.md
#[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
pub struct PackageMetadata {
    pub name: String,
    // contains the version number for the latest version
    #[serde(rename = "dist-tags")]
    pub dist_tags: HashMap<String, String>,
    // publication dates for each version of the package. Used in `old`
    pub time: HashMap<String, String>,
}

/// We use this enum to parse `deprecated` field wether string or bool
///
/// See [this thread](https://users.rust-lang.org/t/how-to-use-multiple-types-for-a-field-in-serde-json/36714/3).
#[derive(Clone, Deserialize, Debug, PartialEq, Eq)]
#[serde(untagged)]
pub enum DeprecatedField {
    String(String),
    Bool(bool)
}

/// JSON Object returned by a `GET https://registry.npmjs.org/:package/:version` request to the [npm registry].
///
/// [npm registry]: https://github.com/npm/registry/blob/master/docs/responses/package-metadata.md
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct VersionObject {
    pub name: String,
    pub version: String,
    // either a deprecation message, or a boolean. Used in `deprecated`
    // using an enum to allow polymorphic parsing, for string or boolean
    pub deprecated: Option<DeprecatedField>,
}

/// `GET https://registry.npmjs.org/:package`
pub async fn pkg_info(pkg_name: &str) -> Result<PackageMetadata, Box<dyn Error>> {
    let resp = reqwest::get(format!("https://registry.npmjs.org/{pkg_name}")).await?;

    match serde_json::from_str(resp.text().await?.as_str()) {
        Ok(json) => Ok(json),
        Err(e) => Err(Box::<dyn Error>::from(format!("{pkg_name}: {e}"))),
    }
}

/// `GET https://registry.npmjs.org/:package/:version`
pub async fn pkg_version_info(
    pkg_name: &str,
    version: &str,
) -> Result<VersionObject, Box<dyn Error>> {
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
