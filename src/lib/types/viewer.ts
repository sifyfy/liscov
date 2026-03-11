// ビューワー管理関連の型定義
// Rust型は generated/ から re-export、フロントエンド固有型はここで定義

// GuiViewerProfile を ViewerProfile として re-export（フロントエンドの命名慣習に合わせる）
export type { GuiViewerProfile as ViewerProfile } from './generated/GuiViewerProfile';
// GuiViewerWithInfo を ViewerWithCustomInfo として re-export
export type { GuiViewerWithInfo as ViewerWithCustomInfo } from './generated/GuiViewerWithInfo';
// GuiContributorStats を ContributorStats として re-export
export type { GuiContributorStats as ContributorStats } from './generated/GuiContributorStats';
// GuiBroadcasterChannel を BroadcasterChannel として re-export
export type { GuiBroadcasterChannel as BroadcasterChannel } from './generated/GuiBroadcasterChannel';

// Session（DBから取得、フロントエンド固有定義）
export interface Session {
  id: string;
  start_time: string;
  end_time: string | null;
  stream_url: string | null;
  stream_title: string | null;
  broadcaster_name: string | null;
  total_messages: number;
  total_revenue: number;
}

// ViewerCustomInfo（インライン情報、フロントエンド固有）
export interface ViewerCustomInfo {
  viewer_profile_id: number;
  reading: string | null;
  notes: string | null;
  custom_data: string | null;
}
