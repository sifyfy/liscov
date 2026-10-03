//! 収益分析・エクスポート（07_revenue.md）
//!
//! スーパーチャットの金額は通貨が混ざるので数値計算しない。YouTube の色（tier）で数える。
//! - `tier`: 色から段階を決める
//! - `models`: 集計結果・エクスポートの型（ts-rs でフロントにも出す）
//! - `query`: DB から数える・読む
//! - `export`: CSV / JSON に整えて書き出す

mod export;
mod models;
mod query;
mod tier;

#[cfg(test)]
mod tests;

pub use models::*;
pub use tier::*;

pub(crate) use export::*;
pub(crate) use query::*;
