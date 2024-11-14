use axum_extra::routing::TypedPath;
use serde_derive::{Deserialize, Serialize};
use strum::{AsRefStr, Display, EnumString, IntoStaticStr, VariantNames};

pub trait PathParts {
    fn parts(&self) -> (Option<String>, Option<TpeKind>, Option<String>) {
        (self.repo(), self.tpe(), self.name())
    }

    fn repo(&self) -> Option<String> {
        None
    }

    fn tpe(&self) -> Option<TpeKind> {
        None
    }

    fn name(&self) -> Option<String> {
        None
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Default,
    Display,
    Serialize,
    Deserialize,
    IntoStaticStr,
    AsRefStr,
    VariantNames,
    EnumString,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
#[strum(ascii_case_insensitive)]
pub enum TpeKind {
    Config,
    #[default]
    Data,
    Index,
    Keys,
    Locks,
    Snapshots,
}

impl TpeKind {
    pub fn into_str(self) -> &'static str {
        self.into()
    }
}

// A type safe route with `"/:repo/config"` as its associated path.
#[derive(TypedPath, Deserialize, Debug)]
#[typed_path("/:repo/config")]
pub struct RepositoryConfigPath {
    pub repo: String,
}

impl PathParts for RepositoryConfigPath {
    fn repo(&self) -> Option<String> {
        Some(self.repo.clone())
    }
}

// A type safe route with `"/:repo/"` as its associated path.
#[derive(TypedPath, Deserialize, Debug)]
#[typed_path("/:repo/")]
pub struct RepositoryPath {
    pub repo: String,
}

impl PathParts for RepositoryPath {
    fn repo(&self) -> Option<String> {
        Some(self.repo.clone())
    }
}

// A type safe route with `"/:tpe"` as its associated path.
#[derive(TypedPath, Deserialize, Debug, Copy, Clone)]
#[typed_path("/:tpe")]
pub struct TpePath {
    pub tpe: TpeKind,
}

impl PathParts for TpePath {
    fn tpe(&self) -> Option<TpeKind> {
        Some(self.tpe)
    }
}

// A type safe route with `"/:repo/:tpe/"` as its associated path.
#[derive(TypedPath, Deserialize, Debug)]
#[typed_path("/:repo/:tpe/")]
pub struct RepositoryTpePath {
    pub repo: String,
    pub tpe: TpeKind,
}

impl PathParts for RepositoryTpePath {
    fn repo(&self) -> Option<String> {
        Some(self.repo.clone())
    }

    fn tpe(&self) -> Option<TpeKind> {
        Some(self.tpe)
    }
}

// A type safe route with `"/:tpe/:name"` as its associated path.
#[derive(TypedPath, Deserialize, Debug)]
#[typed_path("/:tpe/:name")]
pub struct TpeNamePath {
    pub tpe: TpeKind,
    pub name: String,
}

impl PathParts for TpeNamePath {
    fn tpe(&self) -> Option<TpeKind> {
        Some(self.tpe)
    }

    fn name(&self) -> Option<String> {
        Some(self.name.clone())
    }
}

// A type safe route with `"/:repo/:tpe/:name"` as its associated path.
#[derive(TypedPath, Deserialize, Debug)]
#[typed_path("/:repo/:tpe/:name")]
pub struct RepositoryTpeNamePath {
    pub repo: String,
    pub tpe: TpeKind,
    pub name: String,
}

impl PathParts for RepositoryTpeNamePath {
    fn repo(&self) -> Option<String> {
        Some(self.repo.clone())
    }

    fn tpe(&self) -> Option<TpeKind> {
        Some(self.tpe)
    }

    fn name(&self) -> Option<String> {
        Some(self.name.clone())
    }
}
