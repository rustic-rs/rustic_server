use crate::error::ErrorKind;
use crate::error::Result;
use std::fmt::{Display, Formatter};

//pub(crate) const DEFAULT_PATH: &str = "";

// TPE_LOCKS is is defined, but outside this types[] array.
// This allow us to loop over the types[] when generating "routes"
pub(crate) const TPE_DATA: &str = "data";
pub(crate) const TPE_KEYS: &str = "keys";
pub(crate) const TPE_LOCKS: &str = "locks";
pub(crate) const TPE_SNAPSHOTS: &str = "snapshots";
pub(crate) const TPE_INDEX: &str = "index";
pub(crate) const TPE_CONFIG: &str = "config";
pub(crate) const TYPES: [&str; 5] = [TPE_DATA, TPE_KEYS, TPE_LOCKS, TPE_SNAPSHOTS, TPE_INDEX];

#[derive(Debug, PartialEq)]
pub(crate) enum ArchivePathEnum {
    Data,
    Keys,
    Locks,
    Snapshots,
    Index,
    Config,
    None,
}

pub(crate) struct ArchivePath {
    pub(crate) path_type: ArchivePathEnum,
    pub(crate) tpe: String,
    pub(crate) path: String,
    pub(crate) name: String,
}

impl Display for ArchivePath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[ArchivePath] path_type = {:?}, path: {}, tpe: {}, name: {:?}",
            self.path_type, self.path, self.tpe, self.name,
        )
    }
}

pub(crate) fn decompose_path(path: &str) -> Result<ArchivePath> {
    tracing::debug!("[decompose_path] received path: {}", &path);

    // Collect to a list of non empty path elements
    let mut elem: Vec<String> = path
        .split('/')
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let length = elem.len();
    tracing::debug!("[decompose_path] elem = {:?}", &elem);

    let mut ap = ArchivePath {
        path_type: ArchivePathEnum::None,
        tpe: "".to_string(),
        path: "".to_string(),
        name: "".to_string(),
    };

    if length == 0 {
        tracing::debug!("[decompose_path] Empty path!");
        return Err(ErrorKind::FilenameNotAllowed(path.into()));
    }

    // Analyse tail of the path to find name and type values
    let tmp = elem.pop().unwrap();
    let (tpe, name) = if tmp.eq(TPE_CONFIG) {
        ap.path_type = ArchivePathEnum::Config;
        if length > 1 {
            let tpe = elem.pop().unwrap();
            if TYPES.contains(&tpe.as_str()) {
                (tpe, tmp) // path = /:path/:tpe/:config
            } else {
                elem.push(tpe);
                (TPE_CONFIG.to_string(), tmp) // path = /:path/:config
            }
        } else {
            (TPE_CONFIG.to_string(), tmp) // path = /:config
        }
    } else if TYPES.contains(&tmp.as_str()) {
        ap.path_type = get_path_type(&tmp);
        (tmp, "".to_string()) // path = /:path/:tpe --> but NOT "config"
    } else if length > 1 {
        let tpe = elem.pop().unwrap();
        if TYPES.contains(&tpe.as_str()) {
            assert_ne!(tpe.as_str(), TPE_CONFIG); // not allowed: path = /:path/:config/:name
            ap.path_type = get_path_type(&tpe);
            (tpe, tmp) // path = /:path/:tpe/:name
        } else {
            ap.path_type = ArchivePathEnum::None;
            elem.push(tpe);
            elem.push(tmp);
            ("".to_string(), "".to_string()) // path = /:path --> with length (>1)
        }
    } else {
        ap.path_type = ArchivePathEnum::None;
        elem.push(tmp);
        ("".to_string(), "".to_string()) // path = /:path --> with length (1)
    };

    ap.tpe = tpe;
    ap.name = name;
    ap.path = elem.join("/");

    tracing::debug!("[decompose_path]  {:}", &ap);

    Ok(ap)
}

fn get_path_type(s: &str) -> ArchivePathEnum {
    match s {
        TPE_CONFIG => ArchivePathEnum::Config,
        TPE_DATA => ArchivePathEnum::Data,
        TPE_KEYS => ArchivePathEnum::Keys,
        TPE_LOCKS => ArchivePathEnum::Locks,
        TPE_SNAPSHOTS => ArchivePathEnum::Snapshots,
        TPE_INDEX => ArchivePathEnum::Index,
        _ => ArchivePathEnum::None,
    }
}

#[cfg(test)]
mod test {
    use crate::error::Result;
    use crate::handlers::path_analysis::ArchivePathEnum::Config;
    use crate::handlers::path_analysis::{decompose_path, TPE_DATA, TPE_LOCKS};
    use crate::test_helpers::init_tracing;

    #[test]
    fn archive_path_struct() -> Result<()> {
        init_tracing();

        let path = "/a/b/data/name";
        let ap = decompose_path(path)?;
        assert_eq!(ap.tpe, TPE_DATA);
        assert_eq!(ap.name, "name".to_string());
        assert_eq!(ap.path, "a/b");

        let path = "/data/name";
        let ap = decompose_path(path)?;
        assert_eq!(ap.tpe, TPE_DATA);
        assert_eq!(ap.name, "name".to_string());
        assert_eq!(ap.path, "");

        let path = "/a/b/locks";
        let ap = decompose_path(path)?;
        assert_eq!(ap.tpe, TPE_LOCKS);
        assert_eq!(ap.name, "".to_string());
        assert_eq!(ap.path, "a/b");

        let path = "/data";
        let ap = decompose_path(path)?;
        assert_eq!(ap.tpe, TPE_DATA);
        assert_eq!(ap.name, "".to_string());
        assert_eq!(ap.path, "");

        let path = "/a/b/data/config";
        let ap = decompose_path(path)?;
        assert_eq!(ap.path_type, Config);
        assert_eq!(ap.tpe, TPE_DATA);
        assert_eq!(ap.name, "config".to_string());
        assert_eq!(ap.path, "a/b");

        // pub(crate) fn check_name(tpe: &str, name: &str) -> Result<impl IntoResponse>
        // requires that we have type config --> keep similar with "old" rustic server implementation
        let path = "/a/b/config";
        let ap = decompose_path(path)?;
        assert_eq!(ap.path_type, Config);
        assert_eq!(ap.tpe, "config".to_string());
        assert_eq!(ap.name, "config".to_string());
        assert_eq!(ap.path, "a/b");

        let path = "/a/config";
        let ap = decompose_path(path)?;
        assert_eq!(ap.path_type, Config);
        assert_eq!(ap.tpe, "config".to_string());
        assert_eq!(ap.name, "config".to_string());
        assert_eq!(ap.path, "a");

        let path = "/config";
        let ap = decompose_path(path)?;
        assert_eq!(ap.path_type, Config);
        assert_eq!(ap.tpe, "config".to_string());
        assert_eq!(ap.name, "config".to_string());
        assert_eq!(ap.path, "");

        Ok(())
    }
}
