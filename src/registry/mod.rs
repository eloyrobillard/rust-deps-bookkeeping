use std::collections::HashMap;
use std::error::Error;

#[cfg(test)]
mod tests;

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

