//! スーパーチャットの段階（07_revenue.md「Tier別集計」）

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// SuperChat tier based on YouTube color scheme
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../src/lib/types/generated/")]
pub enum SuperChatTier {
    Blue,    // Lowest tier (USD $1-2)
    Cyan,    // USD $2-5
    Green,   // USD $5-10
    Yellow,  // USD $10-20
    Orange,  // USD $20-50
    Magenta, // USD $50-100
    Red,     // Highest tier (USD $100-500)
}

/// SuperChat tier statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default, TS)]
#[ts(export, export_to = "../../src/lib/types/generated/")]
pub struct SuperChatTierStats {
    pub tier_red: usize,
    pub tier_magenta: usize,
    pub tier_orange: usize,
    pub tier_yellow: usize,
    pub tier_green: usize,
    pub tier_cyan: usize,
    pub tier_blue: usize,
    // 段階不明（ヘッダー色が表に無い・色が分からない）
    pub tier_unknown: usize,
}

impl SuperChatTierStats {
    /// 1 件のスーパーチャットを数える。段階が分からなければ None
    pub fn record(&mut self, tier: Option<SuperChatTier>) {
        self.add(tier, 1);
    }

    pub fn increment(&mut self, tier: SuperChatTier) {
        self.add(Some(tier), 1);
    }

    /// 同じ段階のスーパーチャットを count 件数える。段階が分からなければ None
    pub fn add(&mut self, tier: Option<SuperChatTier>, count: usize) {
        let slot = match tier {
            Some(SuperChatTier::Red) => &mut self.tier_red,
            Some(SuperChatTier::Magenta) => &mut self.tier_magenta,
            Some(SuperChatTier::Orange) => &mut self.tier_orange,
            Some(SuperChatTier::Yellow) => &mut self.tier_yellow,
            Some(SuperChatTier::Green) => &mut self.tier_green,
            Some(SuperChatTier::Cyan) => &mut self.tier_cyan,
            Some(SuperChatTier::Blue) => &mut self.tier_blue,
            None => &mut self.tier_unknown,
        };
        *slot += count;
    }

    pub fn total(&self) -> usize {
        self.tier_red
            + self.tier_magenta
            + self.tier_orange
            + self.tier_yellow
            + self.tier_green
            + self.tier_cyan
            + self.tier_blue
            + self.tier_unknown
    }
}

/// スーパーチャットのヘッダー背景色と段階（07_revenue.md「Tier別集計」の表。実データで確認した色）
pub(crate) const TIER_HEADER_COLORS: [(&str, SuperChatTier); 7] = [
    ("#1565C0", SuperChatTier::Blue),
    ("#00B8D4", SuperChatTier::Cyan),
    ("#00BFA5", SuperChatTier::Green),
    ("#FFB300", SuperChatTier::Yellow),
    ("#E65100", SuperChatTier::Orange),
    ("#C2185B", SuperChatTier::Magenta),
    ("#D00000", SuperChatTier::Red),
];

/// ヘッダー背景色（`#RRGGBB`）から段階を判定する。表に無い色・色が無ければ None（段階不明）
///
/// 金額からは推定しない（通貨が混ざるため。07_revenue.md「制約・不変条件」）。
pub(crate) fn tier_from_header_color(color: Option<&str>) -> Option<SuperChatTier> {
    let color = color?;
    TIER_HEADER_COLORS
        .iter()
        .find(|(header, _)| header.eq_ignore_ascii_case(color))
        .map(|(_, tier)| *tier)
}
