use std::fmt::{Display, Formatter};
use crate::handlers::path_analysis::ArchivePathEnum::{CONFIG, DATA, INDEX, KEYS, LOCKS, NONE, SNAPSHOTS};
use crate::web::{TPE_CONFIG, TPE_DATA, TPE_INDEX, TPE_KEYS, TPE_LOCKS, TPE_SNAPSHOTS, TYPES};
use crate::error::Result;
use crate::error::ErrorKind;

#[derive(Debug, PartialEq)]
pub(crate) enum ArchivePathEnum { DATA, KEYS, LOCKS, SNAPSHOTS, INDEX, CONFIG, NONE }
pub(crate) struct ArchivePath{
    pub(crate) path_type: ArchivePathEnum,
    pub(crate) tpe: String,
    pub(crate) path: String,
    pub(crate) name: String,
}

impl Display for ArchivePath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[ArchivePath] path_type = {:?}, path: {}, tpe: {}, name: {:?}",
               self.path_type,
               self.path,
               self.tpe,
               self.name,
        )
    }
}

pub(crate)  fn decompose_path(path:String) -> Result<ArchivePath> {

    // Collect to a list of non empty path elements
    let mut elem:Vec<String> = path
        .split('/')
        .map(|s| s.to_string())
        .filter( |s|{!s.is_empty()})
        .collect();
    let length = elem.len();
    tracing::debug!("[decompose_path] elem = {:?}", &elem);

    let mut ap = ArchivePath{
        path_type: NONE,
        tpe: "".to_string(),
        path: "".to_string(),
        name: "".to_string(),
    };

    if length == 0 {
        tracing::debug!("[decompose_path] Empty path!");
        return Err(ErrorKind::FilenameNotAllowed(path));
    }

    // Analyse tail of the path to find name and type values
    let tmp= elem.pop().unwrap();
    let (tpe, name) = if tmp.eq( TPE_CONFIG ) {
        ap.path_type = CONFIG;
        tracing::debug!("[decompose_path] ends with config");
        if length > 1 {
            let tpe = elem.pop().unwrap();
            if TYPES.contains(&tpe.as_str() ) {
                (tpe, tmp)                      // path = /:path/:tpe/:config
            } else {
                elem.push(tpe);
                ("".to_string(), tmp)           // path = /:path/:config
            }
        } else {
            ("".to_string(), tmp)               // path = /:config
        }
    } else if TYPES.contains(&tmp.as_str()) {
        ap.path_type = get_path_type(&tmp);
        (tmp, "".to_string())                 // path = /:path/:tpe --> but NOT "config"
    } else if length>1 {
        let tpe = elem.pop().unwrap();
        if TYPES.contains(&tpe.as_str()) {
            assert_ne!( tpe.as_str(), TPE_CONFIG ); // not allowed: path = /:path/:config/:name
            ap.path_type = get_path_type(&tpe);
            (tpe, tmp)                        // path = /:path/:tpe/:name
        } else {
            ap.path_type = NONE;
            elem.push(tpe);
            elem.push(tmp);
            ("".to_string(), "".to_string())  // path = /:path --> with length (>1)
        }
    } else {
        ap.path_type = NONE;
        elem.push(tmp);
        ("".to_string(), "".to_string())      // path = /:path --> with length (1)
    };

    ap.tpe = tpe;
    ap.name = name;
    ap.path = elem.join("/");

    tracing::debug!("[decompose_path]  {:}", &ap);

    Ok(ap)
}

fn get_path_type(s:&str) -> ArchivePathEnum {
    match s {
        TPE_CONFIG => {CONFIG},
        TPE_DATA => { DATA  },
        TPE_KEYS  => { KEYS },
        TPE_LOCKS => { LOCKS },
        TPE_SNAPSHOTS => { SNAPSHOTS },
        TPE_INDEX => { INDEX },
        _ => {NONE}
    }
}

#[cfg(test)]
mod test {
    use crate::handlers::path_analysis::decompose_path;
    use crate::web::{TPE_CONFIG, TPE_DATA, TPE_LOCKS};
    use crate::error::Result;
    use crate::handlers::path_analysis::ArchivePathEnum::CONFIG;
    use crate::log::init_tracing;


    #[test]
    fn archive_path_struct() -> Result<()>{
        init_tracing();

        let path = "/a/b/data/name".to_string();
        let ap = decompose_path(path)?;
        assert_eq!( ap.tpe,  TPE_DATA );
        assert_eq!( ap.name,  "name".to_string() );
        assert_eq!( ap.path,  "a/b");

        let path = "/data/name".to_string();
        let ap = decompose_path(path)?;
        assert_eq!( ap.tpe,  TPE_DATA );
        assert_eq!( ap.name,  "name".to_string() );
        assert_eq!( ap.path,  "");

        let path = "/a/b/locks".to_string();
        let ap = decompose_path(path)?;
        assert_eq!( ap.tpe,  TPE_LOCKS );
        assert_eq!( ap.name,  "".to_string() );
        assert_eq!( ap.path,  "a/b");

        let path = "/data".to_string();
        let ap = decompose_path(path)?;
        assert_eq!( ap.tpe,  TPE_DATA );
        assert_eq!( ap.name,  "".to_string() );
        assert_eq!( ap.path,  "");

        let path = "/a/b/data/config".to_string();
        let ap = decompose_path(path)?;
        assert_eq!( ap.path_type,  CONFIG );
        assert_eq!( ap.tpe,  TPE_DATA );
        assert_eq!( ap.name,  "config".to_string() );
        assert_eq!( ap.path,  "a/b");

        let path = "/a/b/config".to_string();
        let ap = decompose_path(path)?;
        assert_eq!( ap.path_type,  CONFIG );
        assert_eq!( ap.tpe,  "".to_string() );
        assert_eq!( ap.name,  "config".to_string() );
        assert_eq!( ap.path,  "a/b");

        let path = "/config".to_string();
        let ap = decompose_path(path)?;
        assert_eq!( ap.path_type,  CONFIG );
        assert_eq!( ap.tpe,  "".to_string() );
        assert_eq!( ap.name,  "config".to_string() );
        assert_eq!( ap.path,  "");

        Ok(())
    }
}