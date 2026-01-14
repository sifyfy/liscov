# ADR-001: 認証情報のセキュアストレージ保存

## ステータス

承認

## コンテキスト

従来、YouTube認証情報（Cookie）は `%APPDATA%/liscov/credentials.toml` にプレーンテキストで保存していた。
これはセキュリティ上の懸念があり、悪意のあるソフトウェアやユーザーが認証情報にアクセス可能な状態だった。

## 決定

認証情報をOS標準のセキュアストレージ（Windows Credential Manager）に保存する。

## 理由

### 検討した選択肢

| 選択肢 | メリット | デメリット |
|-------|---------|-----------|
| プレーンテキスト（現状維持） | 実装簡単、デバッグ容易 | セキュリティリスク |
| セキュアストレージ | OS標準の暗号化、他アプリからアクセス困難 | 実装が複雑、OS依存 |
| 独自暗号化 | OS非依存 | 鍵管理が必要、実装複雑 |

### 採用理由

- OS標準のセキュアストレージを使用することで、追加の鍵管理が不要
- `keyring` crateがWindows/macOS/Linuxをサポートしており、クロスプラットフォーム対応が容易
- ユーザーの認証情報を適切に保護することは、メンバー限定配信へのアクセスを扱うアプリとして必須

## 影響

- **変更箇所:** `auth_save_credentials`, `auth_load_credentials`, `auth_delete_credentials` コマンド
- **マイグレーション:** 既存の `credentials.toml` からセキュアストレージへの自動移行が必要
- **フォールバック:** セキュアストレージが利用できない環境では従来のTOML保存にフォールバック
- **依存追加:** `keyring` crate

## 参考

- [keyring crate](https://crates.io/crates/keyring)
- [Windows Credential Manager](https://docs.microsoft.com/en-us/windows/win32/secauthn/credential-manager)
