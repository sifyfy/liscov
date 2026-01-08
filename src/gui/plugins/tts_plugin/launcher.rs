//! TTS アプリケーション起動ユーティリティ
//!
//! 棒読みちゃん/VOICEVOXの自動起動機能

use std::path::Path;
use std::process::{Child, Command};
use std::sync::Mutex;

use super::config::TtsBackendType;

/// liscovが起動したプロセスを追跡
static LAUNCHED_PROCESSES: Mutex<Vec<LaunchedProcess>> = Mutex::new(Vec::new());

/// 起動したプロセス情報
struct LaunchedProcess {
    backend: TtsBackendType,
    child: Child,
}

/// 棒読みちゃんの既定インストールパス候補
const BOUYOMICHAN_PATHS: &[&str] = &[
    r"C:\Program Files\BouyomiChan\BouyomiChan.exe",
    r"C:\Program Files (x86)\BouyomiChan\BouyomiChan.exe",
    r"C:\BouyomiChan\BouyomiChan.exe",
];

/// 棒読みちゃんのプロセス名
const BOUYOMICHAN_PROCESS_NAME: &str = "BouyomiChan.exe";

/// VOICEVOXのプロセス名
const VOICEVOX_PROCESS_NAME: &str = "VOICEVOX.exe";

/// VOICEVOXの既定インストールパス候補を取得
fn get_voicevox_default_paths() -> Vec<String> {
    let mut paths = Vec::new();

    // %LOCALAPPDATA%\Programs\VOICEVOX\VOICEVOX.exe
    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        paths.push(format!(r"{}\Programs\VOICEVOX\VOICEVOX.exe", local_app_data));
    }

    // %USERPROFILE%\AppData\Local\Programs\VOICEVOX\VOICEVOX.exe
    if let Ok(user_profile) = std::env::var("USERPROFILE") {
        paths.push(format!(
            r"{}\AppData\Local\Programs\VOICEVOX\VOICEVOX.exe",
            user_profile
        ));
    }

    // Program Files
    paths.push(r"C:\Program Files\VOICEVOX\VOICEVOX.exe".to_string());
    paths.push(r"C:\Program Files (x86)\VOICEVOX\VOICEVOX.exe".to_string());

    paths
}

/// 実行ファイルを自動検出
pub fn detect_executable(backend: TtsBackendType) -> Option<String> {
    let paths: Vec<String> = match backend {
        TtsBackendType::Bouyomichan => BOUYOMICHAN_PATHS.iter().map(|s| s.to_string()).collect(),
        TtsBackendType::Voicevox => get_voicevox_default_paths(),
        TtsBackendType::None => return None,
    };

    for path in paths {
        if Path::new(&path).exists() {
            tracing::info!("🔍 実行ファイルを検出: {}", path);
            return Some(path);
        }
    }

    tracing::debug!("🔍 実行ファイルが見つかりませんでした: {:?}", backend);
    None
}

/// プロセスが起動中か確認 (Windows)
#[cfg(target_os = "windows")]
pub fn is_process_running(process_name: &str) -> bool {
    use std::os::windows::process::CommandExt;

    // tasklist コマンドでプロセス一覧を取得
    let output = Command::new("tasklist")
        .args(["/FI", &format!("IMAGENAME eq {}", process_name), "/NH"])
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout.contains(process_name)
        }
        Err(_) => false,
    }
}

/// プロセスが起動中か確認 (非Windows)
#[cfg(not(target_os = "windows"))]
pub fn is_process_running(_process_name: &str) -> bool {
    // 非Windows環境では常にfalseを返す
    false
}

/// バックエンドのプロセス名を取得
pub fn get_process_name(backend: TtsBackendType) -> &'static str {
    match backend {
        TtsBackendType::Bouyomichan => BOUYOMICHAN_PROCESS_NAME,
        TtsBackendType::Voicevox => VOICEVOX_PROCESS_NAME,
        TtsBackendType::None => "",
    }
}

/// アプリケーションを起動
/// - 作業ディレクトリは実行ファイルの親ディレクトリに設定
/// - 起動したプロセスを追跡リストに追加
#[cfg(target_os = "windows")]
pub fn launch_application(path: &str, backend: TtsBackendType) -> Result<(), String> {
    use std::os::windows::process::CommandExt;

    let exe_path = Path::new(path);

    if !exe_path.exists() {
        return Err(format!("実行ファイルが見つかりません: {}", path));
    }

    let working_dir = exe_path
        .parent()
        .ok_or_else(|| "無効なパスです".to_string())?;

    tracing::info!("🚀 アプリケーションを起動: {}", path);
    tracing::debug!("  作業ディレクトリ: {:?}", working_dir);

    let child = Command::new(path)
        .current_dir(working_dir)
        .creation_flags(0x08000000) // CREATE_NO_WINDOW (コンソールウィンドウを表示しない)
        .spawn()
        .map_err(|e| format!("起動に失敗しました: {}", e))?;

    // 起動したプロセスを追跡リストに追加
    if let Ok(mut processes) = LAUNCHED_PROCESSES.lock() {
        processes.push(LaunchedProcess { backend, child });
        tracing::debug!("📝 プロセスを追跡リストに追加 (合計: {})", processes.len());
    }

    Ok(())
}

/// アプリケーションを起動 (非Windows)
#[cfg(not(target_os = "windows"))]
pub fn launch_application(path: &str, backend: TtsBackendType) -> Result<(), String> {
    let exe_path = Path::new(path);

    if !exe_path.exists() {
        return Err(format!("実行ファイルが見つかりません: {}", path));
    }

    let working_dir = exe_path
        .parent()
        .ok_or_else(|| "無効なパスです".to_string())?;

    tracing::info!("🚀 アプリケーションを起動: {}", path);

    let child = Command::new(path)
        .current_dir(working_dir)
        .spawn()
        .map_err(|e| format!("起動に失敗しました: {}", e))?;

    // 起動したプロセスを追跡リストに追加
    if let Ok(mut processes) = LAUNCHED_PROCESSES.lock() {
        processes.push(LaunchedProcess { backend, child });
    }

    Ok(())
}

/// バックエンドを起動（既に起動中なら何もしない）
pub fn launch_backend(backend: TtsBackendType, config_path: Option<&str>) -> Result<(), String> {
    if backend == TtsBackendType::None {
        return Ok(());
    }

    let process_name = get_process_name(backend.clone());

    // 既に起動中か確認
    if is_process_running(process_name) {
        tracing::info!("✅ {} は既に起動中です", process_name);
        return Ok(());
    }

    // パスを決定（設定値 → 自動検出）
    let path = match config_path {
        Some(p) if !p.is_empty() => p.to_string(),
        _ => detect_executable(backend.clone())
            .ok_or_else(|| format!("{:?} の実行ファイルが見つかりません", backend))?,
    };

    launch_application(&path, backend)
}

/// liscovが起動したバックエンドを終了する
pub fn terminate_launched_backend(backend: TtsBackendType) {
    if let Ok(mut processes) = LAUNCHED_PROCESSES.lock() {
        // 該当するバックエンドのプロセスを探して終了
        processes.retain_mut(|p| {
            if p.backend == backend {
                tracing::info!("🛑 {}を終了中...", get_process_name(backend.clone()));
                match p.child.kill() {
                    Ok(()) => {
                        let _ = p.child.wait(); // ゾンビプロセス防止
                        tracing::info!("✅ {}を終了しました", get_process_name(backend.clone()));
                    }
                    Err(e) => {
                        tracing::warn!("⚠️ {}の終了に失敗: {}", get_process_name(backend.clone()), e);
                    }
                }
                false // リストから削除
            } else {
                true // 保持
            }
        });
    }
}

/// liscovが起動した全てのバックエンドを終了する
pub fn terminate_all_launched_backends() {
    if let Ok(mut processes) = LAUNCHED_PROCESSES.lock() {
        for p in processes.iter_mut() {
            let name = get_process_name(p.backend.clone());
            tracing::info!("🛑 {}を終了中...", name);
            match p.child.kill() {
                Ok(()) => {
                    let _ = p.child.wait();
                    tracing::info!("✅ {}を終了しました", name);
                }
                Err(e) => {
                    tracing::warn!("⚠️ {}の終了に失敗: {}", name, e);
                }
            }
        }
        processes.clear();
    }
}

/// liscovが特定のバックエンドを起動したかどうか
pub fn was_launched_by_liscov(backend: TtsBackendType) -> bool {
    if let Ok(processes) = LAUNCHED_PROCESSES.lock() {
        processes.iter().any(|p| p.backend == backend)
    } else {
        false
    }
}

/// バックエンドが起動中かどうかを確認
pub fn is_backend_running(backend: TtsBackendType) -> bool {
    if backend == TtsBackendType::None {
        return false;
    }
    let process_name = get_process_name(backend);
    is_process_running(process_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_voicevox_default_paths() {
        let paths = get_voicevox_default_paths();
        assert!(!paths.is_empty());
    }

    #[test]
    fn test_get_process_name() {
        assert_eq!(
            get_process_name(TtsBackendType::Bouyomichan),
            "BouyomiChan.exe"
        );
        assert_eq!(get_process_name(TtsBackendType::Voicevox), "VOICEVOX.exe");
        assert_eq!(get_process_name(TtsBackendType::None), "");
    }
}
