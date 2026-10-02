//! 設定・認証ファイルの書き込み（FEATURE_SPECIFICATION.md「永続化ファイル一覧」）

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// `contents` を `path` に書く
///
/// 同じフォルダの `<名前>.tmp` に書いてディスクに書き出してから置き換えるので、途中で止まっても
/// 前の内容か新しい内容のどちらかが残る。親フォルダが無ければ作る。
pub fn write(path: &Path, contents: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temp = temp_path(path);
    let result = write_and_replace(&temp, path, contents);
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

fn write_and_replace(temp: &Path, path: &Path, contents: &str) -> io::Result<()> {
    let mut file = fs::File::create(temp)?;
    file.write_all(contents.as_bytes())?;
    file.sync_all()?;
    fs::rename(temp, path)
}

fn temp_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".tmp");
    path.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn writes_contents_and_leaves_no_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");

        write(&path, "a = 1\n").unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "a = 1\n");
        assert!(!dir.path().join("config.toml.tmp").exists());
    }

    #[test]
    fn replaces_previous_contents() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "a = 1\n").unwrap();

        write(&path, "a = 2\n").unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "a = 2\n");
    }

    #[test]
    fn creates_parent_directory() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("config.toml");

        write(&path, "a = 1\n").unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "a = 1\n");
    }

    // 書き込みに失敗しても前の内容が残る（空や途中までのファイルにならない）
    #[test]
    fn keeps_previous_contents_when_writing_fails() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "a = 1\n").unwrap();
        // 一時ファイルの場所をディレクトリで塞いで、書き込みを失敗させる
        fs::create_dir(dir.path().join("config.toml.tmp")).unwrap();

        assert!(write(&path, "a = 2\n").is_err());

        assert_eq!(fs::read_to_string(&path).unwrap(), "a = 1\n");
    }
}
