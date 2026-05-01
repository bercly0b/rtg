use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use toml_edit::{value, DocumentMut, Item, Table};

use crate::infra::{error::AppError, storage_layout::StorageLayout};

const CONFIG_FILE_NAME: &str = "config.toml";
const TELEGRAM_TABLE: &str = "telegram";
const API_ID_KEY: &str = "api_id";
const API_HASH_KEY: &str = "api_hash";

pub fn save_telegram_credentials(
    path: Option<&Path>,
    api_id: i32,
    api_hash: &str,
) -> Result<(), AppError> {
    let target = match path {
        Some(p) => p.to_path_buf(),
        None => default_config_path()?,
    };

    ensure_parent_dir(&target)?;

    let mut doc = read_or_empty_document(&target)?;
    set_telegram_credentials(&mut doc, api_id, api_hash);
    write_atomic(&target, doc.to_string().as_bytes())
}

fn default_config_path() -> Result<PathBuf, AppError> {
    Ok(StorageLayout::resolve()?.config_dir.join(CONFIG_FILE_NAME))
}

fn ensure_parent_dir(target: &Path) -> Result<(), AppError> {
    let Some(parent) = target.parent() else {
        return Ok(());
    };
    if parent.as_os_str().is_empty() {
        return Ok(());
    }
    fs::create_dir_all(parent).map_err(|source| AppError::StorageDirCreate {
        path: parent.to_path_buf(),
        source,
    })
}

fn read_or_empty_document(target: &Path) -> Result<DocumentMut, AppError> {
    if !target.exists() {
        return Ok(DocumentMut::new());
    }
    let raw = fs::read_to_string(target).map_err(|source| AppError::ConfigRead {
        path: target.to_path_buf(),
        source,
    })?;
    raw.parse::<DocumentMut>()
        .map_err(|source| AppError::ConfigSerialize {
            path: target.to_path_buf(),
            source,
        })
}

fn set_telegram_credentials(doc: &mut DocumentMut, api_id: i32, api_hash: &str) {
    let entry = doc.entry(TELEGRAM_TABLE).or_insert_with(|| {
        let mut table = Table::new();
        table.set_implicit(false);
        Item::Table(table)
    });

    if !entry.is_table() {
        *entry = Item::Table(Table::new());
    }

    let table = entry
        .as_table_mut()
        .expect("entry replaced with Table immediately above");
    table[API_ID_KEY] = value(i64::from(api_id));
    table[API_HASH_KEY] = value(api_hash);
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), AppError> {
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty());
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| CONFIG_FILE_NAME.to_owned());
    let tmp_name = format!(".{file_name}.tmp");
    let tmp = match parent {
        Some(p) => p.join(&tmp_name),
        None => PathBuf::from(&tmp_name),
    };

    {
        let mut file = fs::File::create(&tmp).map_err(|source| AppError::ConfigWrite {
            path: tmp.clone(),
            source,
        })?;
        file.write_all(bytes)
            .map_err(|source| AppError::ConfigWrite {
                path: tmp.clone(),
                source,
            })?;
        file.sync_all().map_err(|source| AppError::ConfigWrite {
            path: tmp.clone(),
            source,
        })?;
    }

    fs::rename(&tmp, path).map_err(|source| AppError::ConfigWrite {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(path: &Path) -> String {
        fs::read_to_string(path).expect("config must be readable")
    }

    #[test]
    fn writes_fresh_config_when_file_missing() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let target = tmp.path().join("config.toml");

        save_telegram_credentials(Some(&target), 12345, "abcdef0123456789abcdef0123456789")
            .expect("save must succeed");

        let contents = read(&target);
        assert!(contents.contains("[telegram]"));
        assert!(contents.contains("api_id = 12345"));
        assert!(contents.contains("api_hash = \"abcdef0123456789abcdef0123456789\""));
    }

    #[test]
    fn creates_telegram_section_when_absent_in_existing_file() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let target = tmp.path().join("config.toml");
        fs::write(&target, "[logging]\nlevel = \"debug\"\n").expect("seed file");

        save_telegram_credentials(Some(&target), 7, "feedfacefeedfacefeedfacefeedface")
            .expect("save must succeed");

        let contents = read(&target);
        assert!(contents.contains("[logging]"));
        assert!(contents.contains("level = \"debug\""));
        assert!(contents.contains("[telegram]"));
        assert!(contents.contains("api_id = 7"));
        assert!(contents.contains("api_hash = \"feedfacefeedfacefeedfacefeedface\""));
    }

    #[test]
    fn updates_existing_telegram_placeholders() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let target = tmp.path().join("config.toml");
        fs::write(
            &target,
            "[telegram]\napi_id = 0\napi_hash = \"replace-me\"\n",
        )
        .expect("seed file");

        save_telegram_credentials(Some(&target), 99, "0123456789abcdef0123456789abcdef")
            .expect("save must succeed");

        let contents = read(&target);
        assert!(contents.contains("api_id = 99"));
        assert!(contents.contains("api_hash = \"0123456789abcdef0123456789abcdef\""));
        assert!(!contents.contains("replace-me"));
        assert!(!contents.contains("api_id = 0"));
    }

    #[test]
    fn preserves_user_comments_outside_telegram_block() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let target = tmp.path().join("config.toml");
        let original = "# top-level note\n[logging]\n# log level\nlevel = \"debug\"\n";
        fs::write(&target, original).expect("seed file");

        save_telegram_credentials(Some(&target), 1, "00000000000000000000000000000000")
            .expect("save must succeed");

        let contents = read(&target);
        assert!(contents.contains("# top-level note"));
        assert!(contents.contains("# log level"));
        assert!(contents.contains("[logging]"));
        assert!(contents.contains("level = \"debug\""));
    }

    #[test]
    fn preserves_user_comments_inside_telegram_block() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let target = tmp.path().join("config.toml");
        let original =
            "[telegram]\n# my note about api_id\napi_id = 0\napi_hash = \"replace-me\"\n";
        fs::write(&target, original).expect("seed file");

        save_telegram_credentials(Some(&target), 42, "11111111111111111111111111111111")
            .expect("save must succeed");

        let contents = read(&target);
        assert!(
            contents.contains("# my note about api_id"),
            "in-block comment must survive: {contents}"
        );
        assert!(contents.contains("api_id = 42"));
    }

    #[test]
    fn respects_explicit_path_argument() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let target = tmp.path().join("custom-config.toml");

        save_telegram_credentials(Some(&target), 5, "22222222222222222222222222222222")
            .expect("save must succeed");

        assert!(target.exists());
        assert!(read(&target).contains("api_id = 5"));
    }

    #[test]
    fn creates_parent_directories_if_missing() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let target = tmp.path().join("nested").join("dir").join("config.toml");

        save_telegram_credentials(Some(&target), 8, "33333333333333333333333333333333")
            .expect("save must succeed");

        assert!(target.exists());
    }
}
