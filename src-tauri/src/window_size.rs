//! ウィンドウの最小サイズの適用（10_window_state.md）
//!
//! tauri.conf.json の minWidth・minHeight は手でサイズを変えるときにしか効かず、
//! tauri-plugin-window-state は保存されたサイズをそのまま当てはめる。
//! 保存サイズが最小サイズ未満のときは、表示する前にここで最小サイズへ広げる。

use tauri::LogicalSize;

/// 最小サイズ未満の辺を最小サイズまで広げたサイズを返す。広げる必要が無ければ None
#[must_use]
pub fn fit_to_min_size(size: LogicalSize<f64>, min: LogicalSize<f64>) -> Option<LogicalSize<f64>> {
    let fitted = LogicalSize::new(size.width.max(min.width), size.height.max(min.height));
    (fitted != size).then_some(fitted)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: LogicalSize<f64> = LogicalSize {
        width: 540.0,
        height: 600.0,
    };

    // 仕様例: 保存サイズが最小サイズ（540x600）未満 → 最小サイズを適用
    #[test]
    fn smaller_than_min_becomes_min() {
        assert_eq!(
            fit_to_min_size(LogicalSize::new(400.0, 500.0), MIN),
            Some(MIN)
        );
    }

    #[test]
    fn only_the_smaller_side_is_widened() {
        assert_eq!(
            fit_to_min_size(LogicalSize::new(500.0, 800.0), MIN),
            Some(LogicalSize::new(540.0, 800.0))
        );
        assert_eq!(
            fit_to_min_size(LogicalSize::new(1200.0, 400.0), MIN),
            Some(LogicalSize::new(1200.0, 600.0))
        );
    }

    // 仕様例: 最小サイズちょうど・既定値（1200x800）はそのまま
    #[test]
    fn at_or_above_min_is_unchanged() {
        assert_eq!(fit_to_min_size(MIN, MIN), None);
        assert_eq!(fit_to_min_size(LogicalSize::new(1200.0, 800.0), MIN), None);
    }
}
