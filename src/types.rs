use chrono::{DateTime, FixedOffset};

#[derive(Debug, PartialEq, Eq)]
pub struct PkgNameAndVersion(pub PkgName, pub Version);

#[derive(Debug, PartialEq, Eq)]
pub struct OldPkgDetails {
    pub name: PkgName,
    pub version: Version,
    pub date_version: DateTime<FixedOffset>,
    pub age_version: u32,
    pub latest_version: Version,
    pub date_latest_version: DateTime<FixedOffset>,
    pub age_latest_version: u32,
}

pub type PkgName = String;

pub type Version = String;
