use super::*;

#[tokio::test(flavor = "multi_thread")]
async fn should_return_react_info() {
    let react_metadata = pkg_info("react").await;

    assert!(react_metadata.is_ok());

    let react_metadata = react_metadata.unwrap();

    assert_eq!(react_metadata.name, "react");

    assert!(react_metadata.dist_tags.contains_key("latest"));

    assert!(react_metadata.time.contains_key("18.2.0"));
}

#[tokio::test(flavor = "multi_thread")]
async fn pkg_info_can_return_error() {
    let res = pkg_info("ReAcT").await;

    assert!(res.is_err())
}

#[tokio::test(flavor = "multi_thread")]
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

#[tokio::test(flavor = "multi_thread")]
async fn should_return_error_for_fake_version() {
    let res = pkg_version_info("react", "A.8.0").await;

    assert!(res.is_err())
}

#[tokio::test(flavor = "multi_thread")]
async fn should_return_error_for_fake_pkg() {
    let res = pkg_version_info("ReAcT", "0.1.0").await;

    assert!(res.is_err())
}
