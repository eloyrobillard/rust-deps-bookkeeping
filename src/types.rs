#[derive(Debug, PartialEq, Eq)]
pub struct PkgNameAndVersion(pub PkgName, pub Version);

pub type PkgName = String;

pub type Version = String;
