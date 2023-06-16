mod vulnerability_fix;

use vulnerability_fix::*;

#[tokio::main]
async fn main() {
    let pkg1 = pkg_info("react").await;
    print_pkg_info(pkg1);

    let pkg2 = pkg_version_info("react", "0.8.0").await;
    print_pkg_info(pkg2);
}

// Utils
fn print_pkg_info<T: std::fmt::Debug>(pkg: Result<Result<T, serde_json::Error>, reqwest::Error>) {
    match pkg {
        Ok(o) => match o {
            Ok(data) => println!("{:?}", data),
            Err(e) => println!("{:?}", e)
        }
        Err(e) => println!("{:?}", e)
    }
}