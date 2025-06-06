use crate::gui::models::{GuiChatMessage, MessageType};
use chrono::{DateTime, Timelike, Utc};
use serde::{Deserialize, Serialize};

/// 収益分析の主要データ構造
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct RevenueAnalytics {
    /// Super Chat合計金額
    pub super_chat_total: f64,
    /// Super Chat件数
    pub super_chat_count: usize,
    /// Super Chat平均金額
    pub average_super_chat: f64,
    /// メンバーシップ獲得数
    pub membership_gains: usize,
    /// 時間別収益データ
    pub hourly_revenue: Vec<HourlyRevenue>,
    /// 上位貢献者情報
    pub top_contributors: Vec<ContributorInfo>,
    /// 収益トレンドデータ
    pub revenue_trends: TrendData,
    /// リアルタイム集計エンジン
    pub realtime_engine: RealtimeAggregationEngine,
}

/// リアルタイム集計エンジン
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct RealtimeAggregationEngine {
    /// 現在の分あたりの収益
    pub revenue_per_minute: f64,
    /// 現在の分あたりのメッセージ数
    pub messages_per_minute: usize,
    /// 最後の更新時刻
    pub last_update: Option<DateTime<Utc>>,
    /// 分別統計（直近60分）
    pub minute_stats: Vec<MinuteStats>,
    /// Super Chat金額別統計
    pub amount_distribution: AmountDistribution,
}

/// 分別統計
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MinuteStats {
    /// 分
    pub minute: DateTime<Utc>,
    /// Super Chat金額
    pub super_chat_amount: f64,
    /// メンバーシップ数
    pub membership_count: usize,
    /// メッセージ数
    pub message_count: usize,
}

/// Super Chat金額別分布
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct AmountDistribution {
    /// 100円未満
    pub under_100: usize,
    /// 100-500円
    pub range_100_500: usize,
    /// 500-1000円
    pub range_500_1000: usize,
    /// 1000-5000円
    pub range_1000_5000: usize,
    /// 5000円以上
    pub over_5000: usize,
}

/// 時間別収益データ
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HourlyRevenue {
    /// 該当時間
    pub hour: DateTime<Utc>,
    /// Super Chat金額
    pub super_chat_amount: f64,
    /// メンバーシップ数
    pub membership_count: usize,
    /// メッセージ数
    pub message_count: usize,
}

/// 貢献者情報
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContributorInfo {
    /// チャンネルID
    pub channel_id: String,
    /// 表示名
    pub display_name: String,
    /// 総貢献額
    pub total_contribution: f64,
    /// 貢献回数
    pub contribution_count: usize,
    /// 最後の貢献時刻
    pub last_contribution: DateTime<Utc>,
}

/// トレンドデータ
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct TrendData {
    /// 日別収益データ
    pub daily_trends: Vec<DailyTrend>,
    /// 成長率
    pub growth_rate: f64,
    /// ピーク時間帯
    pub peak_hours: Vec<u8>,
}

/// 日別トレンド
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DailyTrend {
    /// 日付
    pub date: DateTime<Utc>,
    /// その日の総収益
    pub total_revenue: f64,
    /// メッセージ数
    pub message_count: usize,
}

impl RevenueAnalytics {
    /// 新しいRevenueAnalyticsインスタンスを作成
    pub fn new() -> Self {
        Self::default()
    }

    /// メッセージから収益データを更新（リアルタイム集計付き）
    pub fn update_from_message(&mut self, message: &GuiChatMessage) {
        match &message.message_type {
            MessageType::SuperChat { amount } => {
                if let Ok(amount_value) = self.parse_amount(amount) {
                    self.add_super_chat(amount_value, message);
                    // リアルタイム集計エンジンに通知
                    self.realtime_engine.process_super_chat(amount_value);
                }
            }
            MessageType::SuperSticker { amount } => {
                if let Ok(amount_value) = self.parse_amount(amount) {
                    self.add_super_chat(amount_value, message);
                    self.realtime_engine.process_super_chat(amount_value);
                }
            }
            MessageType::Membership => {
                self.add_membership(message);
                self.realtime_engine.process_membership();
            }
            _ => {
                // 通常メッセージもカウント
                self.realtime_engine.process_message();
            }
        }
    }

    /// Super Chat追加
    pub fn add_super_chat(&mut self, amount: f64, message: &GuiChatMessage) {
        self.super_chat_total += amount;
        self.super_chat_count += 1;
        self.average_super_chat = if self.super_chat_count > 0 {
            self.super_chat_total / self.super_chat_count as f64
        } else {
            0.0
        };

        // 貢献者情報更新
        self.update_contributor(message, amount);

        // 時間別データ更新
        self.update_hourly_revenue(amount, 0, 1);
    }

    /// メンバーシップ追加
    pub fn add_membership(&mut self, message: &GuiChatMessage) {
        self.membership_gains += 1;

        // 貢献者情報更新（メンバーシップは金額なしとして扱う）
        self.update_contributor(message, 0.0);

        // 時間別データ更新
        self.update_hourly_revenue(0.0, 1, 1);
    }

    /// 貢献者情報を更新
    fn update_contributor(&mut self, message: &GuiChatMessage, amount: f64) {
        if let Some(contributor) = self
            .top_contributors
            .iter_mut()
            .find(|c| c.channel_id == message.channel_id)
        {
            contributor.total_contribution += amount;
            contributor.contribution_count += 1;
            contributor.last_contribution = Utc::now();
        } else {
            let new_contributor = ContributorInfo {
                channel_id: message.channel_id.clone(),
                display_name: message.author.clone(),
                total_contribution: amount,
                contribution_count: 1,
                last_contribution: Utc::now(),
            };
            self.top_contributors.push(new_contributor);
        }

        // 貢献額でソート（降順）
        self.top_contributors.sort_by(|a, b| {
            b.total_contribution
                .partial_cmp(&a.total_contribution)
                .unwrap()
        });

        // 上位10人に制限
        if self.top_contributors.len() > 10 {
            self.top_contributors.truncate(10);
        }
    }

    /// 時間別収益データを更新
    fn update_hourly_revenue(
        &mut self,
        amount: f64,
        membership_count: usize,
        message_count: usize,
    ) {
        let current_hour = Utc::now()
            .date_naive()
            .and_hms_opt(Utc::now().hour(), 0, 0)
            .unwrap()
            .and_utc();

        if let Some(hourly) = self
            .hourly_revenue
            .iter_mut()
            .find(|h| h.hour == current_hour)
        {
            hourly.super_chat_amount += amount;
            hourly.membership_count += membership_count;
            hourly.message_count += message_count;
        } else {
            let new_hourly = HourlyRevenue {
                hour: current_hour,
                super_chat_amount: amount,
                membership_count,
                message_count,
            };
            self.hourly_revenue.push(new_hourly);
        }

        // 古いデータを削除（24時間分のみ保持）
        let cutoff_time = Utc::now() - chrono::Duration::hours(24);
        self.hourly_revenue.retain(|h| h.hour > cutoff_time);
    }

    /// 金額文字列をパース
    fn parse_amount(&self, amount_str: &str) -> Result<f64, std::num::ParseFloatError> {
        // "¥100", "$5.00", "€3.50" などの形式に対応
        let clean_amount = amount_str
            .chars()
            .filter(|c| c.is_ascii_digit() || *c == '.')
            .collect::<String>();

        clean_amount.parse::<f64>()
    }

    /// 総収益を取得
    pub fn total_revenue(&self) -> f64 {
        self.super_chat_total
    }

    /// 統計サマリーを取得
    pub fn get_summary(&self) -> RevenueSummary {
        RevenueSummary {
            total_revenue: self.super_chat_total,
            super_chat_count: self.super_chat_count,
            average_super_chat: self.average_super_chat,
            membership_gains: self.membership_gains,
            top_contributor: self.top_contributors.first().cloned(),
        }
    }

    /// リアルタイム統計を取得
    pub fn get_realtime_stats(&self) -> RealtimeStats {
        self.realtime_engine.get_current_stats()
    }
}

/// 収益サマリー
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RevenueSummary {
    pub total_revenue: f64,
    pub super_chat_count: usize,
    pub average_super_chat: f64,
    pub membership_gains: usize,
    pub top_contributor: Option<ContributorInfo>,
}

impl RealtimeAggregationEngine {
    /// Super Chat処理
    pub fn process_super_chat(&mut self, amount: f64) {
        self.update_minute_stats(amount, 0, 1);
        self.update_amount_distribution(amount);
        self.calculate_rates();
    }

    /// メンバーシップ処理
    pub fn process_membership(&mut self) {
        self.update_minute_stats(0.0, 1, 1);
        self.calculate_rates();
    }

    /// 通常メッセージ処理
    pub fn process_message(&mut self) {
        self.update_minute_stats(0.0, 0, 1);
        self.calculate_rates();
    }

    /// 分別統計を更新
    fn update_minute_stats(&mut self, amount: f64, membership_count: usize, message_count: usize) {
        let current_minute = Utc::now()
            .date_naive()
            .and_hms_opt(Utc::now().hour(), Utc::now().minute(), 0)
            .unwrap()
            .and_utc();

        if let Some(minute_stat) = self
            .minute_stats
            .iter_mut()
            .find(|m| m.minute == current_minute)
        {
            minute_stat.super_chat_amount += amount;
            minute_stat.membership_count += membership_count;
            minute_stat.message_count += message_count;
        } else {
            let new_minute_stat = MinuteStats {
                minute: current_minute,
                super_chat_amount: amount,
                membership_count,
                message_count,
            };
            self.minute_stats.push(new_minute_stat);
        }

        // 古いデータを削除（60分分のみ保持）
        let cutoff_time = Utc::now() - chrono::Duration::minutes(60);
        self.minute_stats.retain(|m| m.minute > cutoff_time);
    }

    /// 金額分布を更新
    fn update_amount_distribution(&mut self, amount: f64) {
        match amount {
            a if a < 100.0 => self.amount_distribution.under_100 += 1,
            a if a < 500.0 => self.amount_distribution.range_100_500 += 1,
            a if a < 1000.0 => self.amount_distribution.range_500_1000 += 1,
            a if a < 5000.0 => self.amount_distribution.range_1000_5000 += 1,
            _ => self.amount_distribution.over_5000 += 1,
        }
    }

    /// レート計算
    fn calculate_rates(&mut self) {
        let now = Utc::now();

        // 直近1分間の統計を計算
        let one_minute_ago = now - chrono::Duration::minutes(1);
        let recent_stats: Vec<&MinuteStats> = self
            .minute_stats
            .iter()
            .filter(|m| m.minute > one_minute_ago)
            .collect();

        if !recent_stats.is_empty() {
            self.revenue_per_minute = recent_stats
                .iter()
                .map(|s| s.super_chat_amount)
                .sum::<f64>()
                / recent_stats.len() as f64;

            self.messages_per_minute =
                recent_stats.iter().map(|s| s.message_count).sum::<usize>() / recent_stats.len();
        }

        self.last_update = Some(now);
    }

    /// 現在の統計を取得
    pub fn get_current_stats(&self) -> RealtimeStats {
        RealtimeStats {
            revenue_per_minute: self.revenue_per_minute,
            messages_per_minute: self.messages_per_minute,
            amount_distribution: self.amount_distribution.clone(),
            last_update: self.last_update,
        }
    }
}

/// リアルタイム統計
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RealtimeStats {
    pub revenue_per_minute: f64,
    pub messages_per_minute: usize,
    pub amount_distribution: AmountDistribution,
    pub last_update: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::models::{GuiChatMessage, MessageType};

    #[test]
    fn test_revenue_calculation() {
        let mut analytics = RevenueAnalytics::new();

        let super_chat_msg = GuiChatMessage {
            timestamp: "12:00:00".to_string(),
            message_type: MessageType::SuperChat {
                amount: "¥100".to_string(),
            },
            author: "TestUser".to_string(),
            channel_id: "test123".to_string(),
            content: "Thank you!".to_string(),
            metadata: None,
            is_member: false,
        };

        analytics.update_from_message(&super_chat_msg);
        assert_eq!(analytics.total_revenue(), 100.0);
        assert_eq!(analytics.super_chat_count, 1);
        assert_eq!(analytics.average_super_chat, 100.0);
    }

    #[test]
    fn test_multiple_super_chats() {
        let mut analytics = RevenueAnalytics::new();

        let msg1 = GuiChatMessage {
            timestamp: "12:00:00".to_string(),
            message_type: MessageType::SuperChat {
                amount: "¥100".to_string(),
            },
            author: "User1".to_string(),
            channel_id: "user1".to_string(),
            content: "Thanks!".to_string(),
            metadata: None,
            is_member: false,
        };

        let msg2 = GuiChatMessage {
            timestamp: "12:01:00".to_string(),
            message_type: MessageType::SuperChat {
                amount: "¥50".to_string(),
            },
            author: "User2".to_string(),
            channel_id: "user2".to_string(),
            content: "Great stream!".to_string(),
            metadata: None,
            is_member: false,
        };

        analytics.update_from_message(&msg1);
        analytics.update_from_message(&msg2);

        assert_eq!(analytics.total_revenue(), 150.0);
        assert_eq!(analytics.super_chat_count, 2);
        assert_eq!(analytics.average_super_chat, 75.0);
        assert_eq!(analytics.top_contributors.len(), 2);
    }

    #[test]
    fn test_membership_tracking() {
        let mut analytics = RevenueAnalytics::new();

        let membership_msg = GuiChatMessage {
            timestamp: "12:00:00".to_string(),
            message_type: MessageType::Membership,
            author: "NewMember".to_string(),
            channel_id: "member123".to_string(),
            content: "New member!".to_string(),
            metadata: None,
            is_member: true,
        };

        analytics.update_from_message(&membership_msg);
        assert_eq!(analytics.membership_gains, 1);
    }

    #[test]
    fn test_realtime_aggregation_engine() {
        let mut analytics = RevenueAnalytics::new();

        // Super Chat処理テスト
        let super_chat_msg = GuiChatMessage {
            timestamp: "12:00:00".to_string(),
            message_type: MessageType::SuperChat {
                amount: "¥500".to_string(),
            },
            author: "TestUser".to_string(),
            channel_id: "test123".to_string(),
            content: "Thank you!".to_string(),
            metadata: None,
            is_member: false,
        };

        analytics.update_from_message(&super_chat_msg);

        let realtime_stats = analytics.get_realtime_stats();
        assert!(realtime_stats.last_update.is_some());
        assert_eq!(realtime_stats.amount_distribution.range_500_1000, 1);
    }

    #[test]
    fn test_amount_distribution() {
        let mut engine = RealtimeAggregationEngine::default();

        // 各金額帯をテスト
        engine.update_amount_distribution(50.0); // under_100
        engine.update_amount_distribution(200.0); // range_100_500
        engine.update_amount_distribution(750.0); // range_500_1000
        engine.update_amount_distribution(2000.0); // range_1000_5000
        engine.update_amount_distribution(10000.0); // over_5000

        assert_eq!(engine.amount_distribution.under_100, 1);
        assert_eq!(engine.amount_distribution.range_100_500, 1);
        assert_eq!(engine.amount_distribution.range_500_1000, 1);
        assert_eq!(engine.amount_distribution.range_1000_5000, 1);
        assert_eq!(engine.amount_distribution.over_5000, 1);
    }

    #[test]
    fn test_minute_stats_cleanup() {
        let mut engine = RealtimeAggregationEngine::default();

        // 古い統計データを追加
        let old_time = Utc::now() - chrono::Duration::minutes(70);
        engine.minute_stats.push(MinuteStats {
            minute: old_time,
            super_chat_amount: 100.0,
            membership_count: 1,
            message_count: 5,
        });

        // 新しいデータを処理
        engine.process_super_chat(200.0);

        // 古いデータが削除されていることを確認
        assert!(engine
            .minute_stats
            .iter()
            .all(|s| s.minute > Utc::now() - chrono::Duration::minutes(60)));
    }
}
