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

// Session（DB のセッション。database/models.rs）
export type { Session } from './generated/Session';
