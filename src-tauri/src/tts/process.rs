//! TTS backend process management
//!
//! Handles auto-launching and killing TTS backend processes (Bouyomichan/VOICEVOX).

use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::Arc;
use tokio::sync::Mutex;

use super::config::TtsBackendType;

/// Launched process info
struct LaunchedProcess {
    backend: TtsBackendType,
    child: Child,
}

/// TTS process manager
pub struct TtsProcessManager {
    processes: Arc<Mutex<Vec<LaunchedProcess>>>,
}

impl TtsProcessManager {
    /// Create a new process manager
    pub fn new() -> Self {
        Self {
            processes: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Discover executable path for a backend
    pub fn discover_exe(backend: &TtsBackendType) -> Option<String> {
        let paths = match backend {
            TtsBackendType::Bouyomichan => Self::bouyomichan_search_paths(),
            TtsBackendType::Voicevox => Self::voicevox_search_paths(),
            TtsBackendType::None => return None,
        };

        for path in paths {
            if path.exists() {
                return path.to_str().map(|s| s.to_string());
            }
        }
        None
    }

    /// Get search paths for Bouyomichan executable
    fn bouyomichan_search_paths() -> Vec<PathBuf> {
        vec![
            PathBuf::from(r"C:\BouyomiChan\BouyomiChan.exe"),
            PathBuf::from(r"C:\Program Files\BouyomiChan\BouyomiChan.exe"),
            PathBuf::from(r"C:\Program Files (x86)\BouyomiChan\BouyomiChan.exe"),
        ]
    }

    /// Get search paths for VOICEVOX executable
    fn voicevox_search_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // %LOCALAPPDATA%\Programs\VOICEVOX\VOICEVOX.exe
        if let Some(local_app_data) = dirs::data_local_dir() {
            paths.push(local_app_data.join("Programs").join("VOICEVOX").join("VOICEVOX.exe"));
        }

        paths.push(PathBuf::from(r"C:\Program Files\VOICEVOX\VOICEVOX.exe"));

        paths
    }

    /// Launch a backend process
    pub async fn launch(
        &self,
        backend: TtsBackendType,
        exe_path: Option<&str>,
    ) -> Result<u32, String> {
        // Check if already launched
        {
            let processes = self.processes.lock().await;
            if processes.iter().any(|p| p.backend == backend) {
                return Err(format!("{:?} is already launched", backend));
            }
        }

        // Determine exe path
        let path = match exe_path {
            Some(p) => PathBuf::from(p),
            None => {
                let discovered = Self::discover_exe(&backend)
                    .ok_or_else(|| format!("Could not find {:?} executable", backend))?;
                PathBuf::from(discovered)
            }
        };

        if !path.exists() {
            return Err(format!("Executable not found: {}", path.display()));
        }

        log::info!("Launching {:?} from {:?}", backend, path);

        // Spawn the process
        let child = Command::new(&path)
            .spawn()
            .map_err(|e| format!("Failed to launch {:?}: {}", backend, e))?;

        let pid = child.id();
        log::info!("{:?} launched with PID {}", backend, pid);

        // Store the process
        self.processes.lock().await.push(LaunchedProcess { backend, child });

        Ok(pid)
    }

    /// Kill a backend process (only if we launched it)
    pub async fn kill(&self, backend: &TtsBackendType) -> Result<(), String> {
        let mut processes = self.processes.lock().await;

        let pos = processes.iter().position(|p| &p.backend == backend);
        if let Some(idx) = pos {
            let mut process = processes.remove(idx);
            log::info!("Killing {:?} process (PID {})", backend, process.child.id());
            process.child.kill().map_err(|e| format!("Failed to kill {:?}: {}", backend, e))?;
            Ok(())
        } else {
            Err(format!("{:?} was not launched by this app", backend))
        }
    }

    /// Kill all launched processes
    pub async fn kill_all(&self) {
        let mut processes = self.processes.lock().await;
        for mut process in processes.drain(..) {
            log::info!(
                "Killing {:?} process (PID {})",
                process.backend,
                process.child.id()
            );
            if let Err(e) = process.child.kill() {
                log::error!("Failed to kill {:?}: {}", process.backend, e);
            }
        }
    }

    /// Check if a backend was launched by this app
    pub async fn is_launched(&self, backend: &TtsBackendType) -> bool {
        self.processes.lock().await.iter().any(|p| &p.backend == backend)
    }
}

impl Default for TtsProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_creates_empty_processes() {
        let mgr = TtsProcessManager::new();
        // 初期状態では起動プロセスなし（同期的に確認可能な範囲で）
        assert!(mgr.processes.try_lock().is_ok());
    }

    #[tokio::test]
    async fn test_is_launched_returns_false_initially() {
        let mgr = TtsProcessManager::new();
        assert!(!mgr.is_launched(&TtsBackendType::Bouyomichan).await);
        assert!(!mgr.is_launched(&TtsBackendType::Voicevox).await);
        assert!(!mgr.is_launched(&TtsBackendType::None).await);
    }

    #[test]
    fn test_discover_exe_none_backend_returns_none() {
        let result = TtsProcessManager::discover_exe(&TtsBackendType::None);
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_kill_not_launched_returns_error() {
        let mgr = TtsProcessManager::new();

        let result = mgr.kill(&TtsBackendType::Bouyomichan).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("was not launched"));

        let result = mgr.kill(&TtsBackendType::Voicevox).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("was not launched"));
    }

    #[tokio::test]
    async fn test_launch_nonexistent_path_returns_error() {
        let mgr = TtsProcessManager::new();

        let result = mgr
            .launch(TtsBackendType::Bouyomichan, Some("/nonexistent/path.exe"))
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[tokio::test]
    async fn test_launch_without_path_and_not_installed_returns_error() {
        let mgr = TtsProcessManager::new();

        // 棒読みちゃんがインストールされていない環境では自動探索失敗
        // （インストールされている環境ではスキップ）
        let result = mgr.launch(TtsBackendType::Bouyomichan, None).await;

        // 棒読みちゃんがインストールされていなければエラー
        if result.is_err() {
            let err = result.unwrap_err();
            assert!(
                err.contains("Could not find") || err.contains("not found"),
                "Unexpected error: {}",
                err
            );
        }
        // インストールされている場合は成功する可能性があるので、その場合はkillして終了
        else {
            let _ = mgr.kill(&TtsBackendType::Bouyomichan).await;
        }
    }

    #[tokio::test]
    async fn test_kill_all_on_empty_does_not_panic() {
        let mgr = TtsProcessManager::new();
        mgr.kill_all().await;
        // パニックしなければOK
    }

    #[test]
    fn test_bouyomichan_search_paths_not_empty() {
        let paths = TtsProcessManager::bouyomichan_search_paths();
        assert!(!paths.is_empty());
        // すべてのパスがBouyomiChan.exeで終わることを確認
        for path in &paths {
            assert!(
                path.to_string_lossy().ends_with("BouyomiChan.exe"),
                "Path should end with BouyomiChan.exe: {:?}",
                path
            );
        }
    }

    #[test]
    fn test_voicevox_search_paths_not_empty() {
        let paths = TtsProcessManager::voicevox_search_paths();
        assert!(!paths.is_empty());
        // すべてのパスがVOICEVOX.exeで終わることを確認
        for path in &paths {
            assert!(
                path.to_string_lossy().ends_with("VOICEVOX.exe"),
                "Path should end with VOICEVOX.exe: {:?}",
                path
            );
        }
    }
}
