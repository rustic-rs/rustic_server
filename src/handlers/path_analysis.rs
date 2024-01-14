use std::fmt::{Display, Formatter};
use crate::handlers::path_analysis::ArchivePathEnum::{CONFIG, DATA, INDEX, KEYS, LOCKS, NONE, SNAPSHOTS};
use crate::web::{TPE_CONFIG, TPE_DATA, TPE_INDEX, TPE_KEYS, TPE_LOCKS, TPE_SNAPSHOTS, TYPES};


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

pub(crate)  fn decompose_path(path:String) -> ArchivePath {

    // Collect to a list of non empty path elements
    let mut elem:Vec<String> = path
        .split('/')
        .map(|s| s.to_string())
        .filter( |s|{!s.is_empty()})
        .collect();
    let length = elem.len();
    tracing::debug!("elem = {:?}", &elem);

    let mut ap = ArchivePath{
        path_type: NONE,
        tpe: "".to_string(),
        path: "".to_string(),
        name: "".to_string(),
    };

    if length == 0 { return ap; }

    // Analyse tail of the path to find name and type values
    let tmp= elem.pop().unwrap();
    let (tpe, name) = if TYPES.contains(&tmp.as_str()) {
        (tmp, "".to_string())                 // path = /:path/:tpe
    } else if length>1 {
        let tpe = elem.pop().unwrap();
        if TYPES.contains(&tpe.as_str()) {
            (tpe, tmp)                        // path = /:path/:tpe/:name
        } else {
            elem.push(tpe);
            elem.push(tmp);
            ("".to_string(), "".to_string())  // path = /:path
        }
    } else {
        elem.push(tmp);
        ("".to_string(), "".to_string())      // path = /:path
    };

    match tpe.as_str() {
        TPE_CONFIG  => { ap.path_type = CONFIG; },
        TPE_DATA => { ap.path_type = DATA;  },
        TPE_KEYS  => { ap.path_type = KEYS; },
        TPE_LOCKS => { ap.path_type = LOCKS; },
        TPE_SNAPSHOTS => { ap.path_type = SNAPSHOTS; },
        TPE_INDEX => { ap.path_type = INDEX; },
        _ => {}
    };

    ap.tpe = tpe;
    ap.name = name;
    ap.path = elem.join("/");

    ap
}

#[cfg(test)]
mod test {
    use crate::handlers::path_analysis::decompose_path;
    use crate::web::{TPE_CONFIG, TPE_LOCKS};

    #[test]
    fn archive_path_struct() {
        let path = "/a/b/locks/name".to_string();
        let ap = decompose_path(path);
        assert_eq!( ap.tpe,  TPE_LOCKS );
        assert_eq!( ap.name,  "name".to_string() );
        assert_eq!( ap.path,  "a/b");

        let path = "/locks/name".to_string();
        let ap = decompose_path(path);
        assert_eq!( ap.tpe,  TPE_LOCKS );
        assert_eq!( ap.name,  "name".to_string() );
        assert_eq!( ap.path,  "");

        let path = "/a/b/locks".to_string();
        let ap = decompose_path(path);
        assert_eq!( ap.tpe,  TPE_LOCKS );
        assert_eq!( ap.name,  "".to_string() );
        assert_eq!( ap.path,  "a/b");

        let path = "/locks".to_string();
        let ap = decompose_path(path);
        assert_eq!( ap.tpe,  TPE_LOCKS );
        assert_eq!( ap.name,  "".to_string() );
        assert_eq!( ap.path,  "");

        let path = "/a/b/config".to_string();
        let ap = decompose_path(path);
        assert_eq!( ap.tpe,  TPE_CONFIG );
        assert_eq!( ap.name,  "".to_string() );
        assert_eq!( ap.path,  "a/b");

        let path = "/config".to_string();
        let ap = decompose_path(path);
        assert_eq!( ap.tpe,  TPE_CONFIG );
        assert_eq!( ap.name,  "".to_string() );
        assert_eq!( ap.path,  "");
    }
}