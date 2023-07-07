use once_cell::sync::Lazy;

use super::*;

static DEPR_PKG_DETAILS: Lazy<Vec<VersionObject>> = Lazy::new(|| {
    vec![
        VersionObject {
            name: "depr1".to_owned(),
            version: "0.0.1".to_owned(),
            deprecated: Some(DeprecatedField::String("AAAAAAAAAAAAAAAAAA".to_owned())),
        },
        VersionObject {
            name: "depr2".to_owned(),
            version: "0.0.1".to_owned(),
            deprecated: Some(DeprecatedField::Bool(true)),
        },
        VersionObject {
            name: "not_depr".to_owned(),
            version: "0.0.1".to_owned(),
            deprecated: Some(DeprecatedField::Bool(false)),
        },
    ]
});

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn deprecated_test() -> Result<(), Box<dyn Error>> {
    let path = Path::new("./test-assets/monorepo/");

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
        |VersionObject {
             name,
             version,
             deprecated,
         }| name == "@babel/polyfill" && version == "7.12.1" && deprecated.is_some()
    ));

    assert!(dev_depr.is_empty());

    Ok(())
}

#[test]
fn get_deprecated_output_test() {
    let (in1, in2) = (DEPR_PKG_DETAILS.to_vec(), DEPR_PKG_DETAILS.to_vec());

    let output = get_output((&in1, &in2), false);

    assert_eq!(output, "\n  production:\n\n    depr1@0.0.1\n    depr2@0.0.1\n    not_depr@0.0.1\n\n  development:\n\n    depr1@0.0.1\n    depr2@0.0.1\n    not_depr@0.0.1\n\n  total: 3 deprecated dependencies, 3 deprecated dev dependencies\n");

    let output = get_output((&in1, &in2), true);

    assert_eq!(output, "\n  depr1@0.0.1\n  depr2@0.0.1\n  not_depr@0.0.1\n\n  total: 3 deprecated production dependencies\n");
}

#[test]
fn format_deprecated_output_test() {
    let output = get_pkgs_output(DEPR_PKG_DETAILS.as_slice(), None);

    assert_eq!(output, "\n  depr1@0.0.1\n  depr2@0.0.1\n  not_depr@0.0.1\n");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn should_filter_deprecated_pkgs() -> Result<(), Box<dyn Error>> {
    let deps = vec![PkgNameAndVersion("core-js".to_owned(), "3.19.0".to_owned())];

    let res = filter_deprecated(&deps).await?;

    let expected = VersionObject {
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
