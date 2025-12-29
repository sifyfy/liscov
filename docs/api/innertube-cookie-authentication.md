# InnerTube API: Cookie認証 実装ガイド

このドキュメントは、YouTube InnerTube APIでメンバー限定配信のチャットにアクセスするためのCookie認証の実装詳細をまとめたものです。

## 目次

1. [概要](#概要)
2. [必要なCookie](#必要なcookie)
3. [Cookieの取得方法](#cookieの取得方法)
4. [SAPISIDHASH の生成](#sapissidhashの生成)
5. [HTTPリクエストヘッダー](#httpリクエストヘッダー)
6. [Cookieファイル形式](#cookieファイル形式)
7. [Rust実装例](#rust実装例)
8. [トラブルシューティング](#トラブルシューティング)

---

## 概要

YouTube InnerTube APIの認証には、ブラウザセッションから取得したCookieを使用します。この認証により、以下の機能が利用可能になります：

- メンバー限定ライブ配信のチャット取得
- 年齢制限コンテンツへのアクセス
- ユーザー固有の設定・履歴の反映

### 認証の仕組み

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Browser   │────>│   Cookie    │────>│  SAPISIDHASH │
│   Session   │     │  Extraction │     │  Generation  │
└─────────────┘     └─────────────┘     └──────┬───────┘
                                               │
                                               v
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  InnerTube  │<────│   HTTP      │<────│  Authorization│
│    API      │     │   Request   │     │    Header    │
└─────────────┘     └─────────────┘     └─────────────┘
```

---

## 必要なCookie

### 認証に必須のCookie（5種類）

| Cookie名 | 説明 | 用途 |
|----------|------|------|
| `SID` | Session ID | セッション識別 |
| `HSID` | HTTP Secure Session ID | HTTPセキュア認証 |
| `SSID` | Secure Session ID | セキュア認証 |
| `APISID` | API Session ID | API認証 |
| `SAPISID` | Secure API Session ID | **SAPISIDHASH生成に使用** |

### 補助的なCookie（オプション）

| Cookie名 | 説明 |
|----------|------|
| `LOGIN_INFO` | ログイン情報（Base64エンコード） |
| `PREF` | ユーザー設定 |
| `YSC` | セッションクッキー |
| `VISITOR_INFO1_LIVE` | 訪問者情報 |

### Cookie有効期限

- **ブラウザセッション**: 約2年間有効（ログアウトしない限り）
- **注意**: YouTubeは開いているタブでCookieを自動ローテーションする

---

## Cookieの取得方法

### 方法1: ブラウザDevToolsから手動取得（推奨）

#### 手順

1. **プライベート/シークレットウィンドウを開く**
   ```
   Chrome:  Ctrl+Shift+N (Windows) / Cmd+Shift+N (Mac)
   Firefox: Ctrl+Shift+P (Windows) / Cmd+Shift+P (Mac)
   ```
   > 重要: 通常ウィンドウではCookieが自動ローテーションされるため、プライベートウィンドウを使用

2. **YouTubeにログイン**
   - https://www.youtube.com にアクセス
   - メンバーシップを持つアカウントでログイン

3. **DevToolsを開く**
   - `F12` または `Ctrl+Shift+I` (Windows) / `Cmd+Option+I` (Mac)

4. **Networkタブでリクエストをキャプチャ**
   - Networkタブを選択
   - フィルターに `browse` または `youtubei` を入力
   - ページをスクロールしてリクエストを発生させる

5. **Cookieヘッダーをコピー**
   - POSTリクエストを選択
   - Headersタブ → Request Headers → `Cookie` の値をコピー

6. **ブラウザを閉じる**
   - **ログアウトせずに**ウィンドウを閉じる（重要）

#### 取得例

```
SID=xxx; HSID=xxx; SSID=xxx; APISID=xxx; SAPISID=xxx; ...
```

### 方法2: ブラウザ拡張機能を使用

#### Chrome

- [Get cookies.txt LOCALLY](https://chromewebstore.google.com/detail/get-cookiestxt-locally/cclelndahbckbenkjhflpdbgdldlbecc)
- [Get cookies.txt Clean](https://chromewebstore.google.com/detail/get-cookiestxt-clean/ahmnmhfbokciafffnknlekllgcnafnie)

> 警告: オリジナルの「Get cookies.txt」はマルウェアとして2023年3月に削除されました

#### Firefox

- [cookies.txt](https://addons.mozilla.org/en-US/firefox/addon/cookies-txt/)

### 方法3: yt-dlp を使用（プログラマティック）

```bash
# ブラウザからCookieを抽出してファイルに保存
yt-dlp --cookies-from-browser firefox --cookies cookies.txt "https://www.youtube.com"

# Chrome使用時（ブラウザを閉じる必要あり）
yt-dlp --cookies-from-browser chrome --cookies cookies.txt "https://www.youtube.com"
```

#### サポートされるブラウザ

| ブラウザ | オプション値 |
|----------|-------------|
| Chrome | `chrome` |
| Firefox | `firefox` |
| Edge | `edge` |
| Brave | `brave` |
| Opera | `opera` |
| Chromium | `chromium` |
| Safari | `safari` |
| Vivaldi | `vivaldi` |

#### プロファイル指定

```bash
# 特定のプロファイルを指定
yt-dlp --cookies-from-browser "firefox:Profile1" --cookies cookies.txt URL

# Firefoxコンテナ指定
yt-dlp --cookies-from-browser "firefox:default::Container Name" --cookies cookies.txt URL

# Linuxでキーリング指定
yt-dlp --cookies-from-browser "chrome::gnomekeyring" --cookies cookies.txt URL
```

### 方法4: Cookieファイルからの読み込み

既存のNetscape形式cookies.txtファイルから必要なCookieを抽出：

```python
def parse_cookies_txt(file_path: str) -> dict:
    cookies = {}
    required = ['SID', 'HSID', 'SSID', 'APISID', 'SAPISID']

    with open(file_path, 'r') as f:
        for line in f:
            if line.startswith('#') or not line.strip():
                continue
            fields = line.strip().split('\t')
            if len(fields) >= 7:
                name, value = fields[5], fields[6]
                if name in required:
                    cookies[name] = value

    return cookies
```

### 方法5: ブラウザデータベースから直接抽出（プログラマティック）

ブラウザのCookieデータベース（SQLite）から直接Cookieを抽出する方法です。

#### ブラウザ別データベースの場所

**Chrome / Chromium系**

| OS | パス |
|----|------|
| Windows | `%LOCALAPPDATA%\Google\Chrome\User Data\Default\Network\Cookies` |
| macOS | `~/Library/Application Support/Google/Chrome/Default/Cookies` |
| Linux | `~/.config/google-chrome/Default/Cookies` |

**Firefox**

| OS | パス |
|----|------|
| Windows | `%APPDATA%\Mozilla\Firefox\Profiles\<profile>\cookies.sqlite` |
| macOS | `~/Library/Application Support/Firefox/Profiles/<profile>/cookies.sqlite` |
| Linux | `~/.mozilla/firefox/<profile>/cookies.sqlite` |

#### Chrome Cookie の暗号化

ChromeはCookie値を暗号化して保存しています。復号化にはOS固有の処理が必要です。

**暗号化方式**:
- **Windows**: DPAPI (Data Protection API) + AES-256-GCM
- **macOS**: Keychain + AES-128-CBC
- **Linux**: GNOME Keyring または KWallet + AES-128-CBC

**復号化キーの取得**:

```
Windows: %LOCALAPPDATA%\Google\Chrome\User Data\Local State
         → os_crypt.encrypted_key をDPAPIで復号化

macOS:   Keychainから "Chrome Safe Storage" キーを取得

Linux:   GNOME Keyring または KWallet から "Chrome Safe Storage" を取得
```

**暗号化データ構造** (Chrome v80以降):

```
+--------+----+---------------------------+-----+
| v10/v11| IV |     Encrypted Data        | Tag |
| 3 bytes|12B |       Variable            | 16B |
+--------+----+---------------------------+-----+
```

#### Firefox Cookie（暗号化なし）

FirefoxはCookieを暗号化せずにSQLiteに保存しますが、**データベースファイルがロック**されます。

**テーブル構造** (`moz_cookies`):

```sql
CREATE TABLE moz_cookies (
    id INTEGER PRIMARY KEY,
    baseDomain TEXT,
    originAttributes TEXT NOT NULL DEFAULT '',
    name TEXT,
    value TEXT,
    host TEXT,
    path TEXT,
    expiry INTEGER,
    lastAccessed INTEGER,
    creationTime INTEGER,
    isSecure INTEGER,
    isHttpOnly INTEGER,
    inBrowserElement INTEGER DEFAULT 0,
    sameSite INTEGER DEFAULT 0,
    rawSameSite INTEGER DEFAULT 0,
    schemeMap INTEGER DEFAULT 0
);
```

#### Rust実装: Firefox Cookie抽出

```rust
use rusqlite::{Connection, Result};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Firefox Cookieを抽出する
pub struct FirefoxCookieExtractor {
    profile_path: PathBuf,
}

impl FirefoxCookieExtractor {
    pub fn new(profile_path: PathBuf) -> Self {
        Self { profile_path }
    }

    /// デフォルトプロファイルを自動検出
    #[cfg(target_os = "windows")]
    pub fn find_default_profile() -> Option<PathBuf> {
        let appdata = std::env::var("APPDATA").ok()?;
        let profiles_dir = PathBuf::from(appdata)
            .join("Mozilla")
            .join("Firefox")
            .join("Profiles");

        Self::find_default_in_dir(&profiles_dir)
    }

    #[cfg(target_os = "macos")]
    pub fn find_default_profile() -> Option<PathBuf> {
        let home = std::env::var("HOME").ok()?;
        let profiles_dir = PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("Firefox")
            .join("Profiles");

        Self::find_default_in_dir(&profiles_dir)
    }

    #[cfg(target_os = "linux")]
    pub fn find_default_profile() -> Option<PathBuf> {
        let home = std::env::var("HOME").ok()?;
        let profiles_dir = PathBuf::from(home)
            .join(".mozilla")
            .join("firefox");

        Self::find_default_in_dir(&profiles_dir)
    }

    fn find_default_in_dir(profiles_dir: &PathBuf) -> Option<PathBuf> {
        fs::read_dir(profiles_dir).ok()?.find_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".default") || name.ends_with(".default-release") {
                Some(entry.path())
            } else {
                None
            }
        })
    }

    /// YouTube用のCookieを抽出
    pub fn extract_youtube_cookies(&self) -> Result<HashMap<String, String>> {
        let db_path = self.profile_path.join("cookies.sqlite");

        // Firefoxがロックしているため、一時ファイルにコピー
        let temp_path = std::env::temp_dir().join("liscov_cookies_temp.sqlite");
        fs::copy(&db_path, &temp_path)
            .map_err(|e| rusqlite::Error::InvalidPath(temp_path.clone()))?;

        let conn = Connection::open(&temp_path)?;

        let mut stmt = conn.prepare(
            "SELECT name, value FROM moz_cookies
             WHERE host LIKE '%youtube.com' OR host LIKE '%google.com'
             AND name IN ('SID', 'HSID', 'SSID', 'APISID', 'SAPISID')"
        )?;

        let mut cookies = HashMap::new();
        let mut rows = stmt.query([])?;

        while let Some(row) = rows.next()? {
            let name: String = row.get(0)?;
            let value: String = row.get(1)?;
            cookies.insert(name, value);
        }

        // 一時ファイルを削除
        let _ = fs::remove_file(&temp_path);

        Ok(cookies)
    }
}
```

#### Rust実装: Chrome Cookie抽出 (Windows)

```rust
#[cfg(target_os = "windows")]
pub mod chrome_windows {
    use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
    use aes_gcm::aead::Aead;
    use base64::{Engine, engine::general_purpose};
    use rusqlite::Connection;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use windows::Win32::Security::Cryptography::{
        CryptUnprotectData, CRYPT_INTEGER_BLOB,
    };

    /// Chrome暗号化キーを取得
    fn get_encryption_key() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let local_appdata = std::env::var("LOCALAPPDATA")?;
        let local_state_path = PathBuf::from(&local_appdata)
            .join("Google")
            .join("Chrome")
            .join("User Data")
            .join("Local State");

        let content = std::fs::read_to_string(&local_state_path)?;
        let json: serde_json::Value = serde_json::from_str(&content)?;

        let encrypted_key_b64 = json["os_crypt"]["encrypted_key"]
            .as_str()
            .ok_or("encrypted_key not found")?;

        let encrypted_key = general_purpose::STANDARD.decode(encrypted_key_b64)?;

        // "DPAPI" プレフィックス (5バイト) を除去
        let encrypted_key = &encrypted_key[5..];

        // DPAPIで復号化
        let decrypted = dpapi_decrypt(encrypted_key)?;

        Ok(decrypted)
    }

    /// DPAPIで復号化
    fn dpapi_decrypt(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        unsafe {
            let mut input = CRYPT_INTEGER_BLOB {
                cbData: data.len() as u32,
                pbData: data.as_ptr() as *mut u8,
            };

            let mut output = CRYPT_INTEGER_BLOB::default();

            CryptUnprotectData(
                &mut input,
                None,
                None,
                None,
                None,
                0,
                &mut output,
            )?;

            let decrypted = std::slice::from_raw_parts(
                output.pbData,
                output.cbData as usize
            ).to_vec();

            Ok(decrypted)
        }
    }

    /// Cookie値を復号化 (AES-256-GCM)
    fn decrypt_cookie_value(
        encrypted_value: &[u8],
        key: &[u8],
    ) -> Result<String, Box<dyn std::error::Error>> {
        // v10/v11 プレフィックスをチェック
        if encrypted_value.len() < 15 {
            return Err("Invalid encrypted value".into());
        }

        let prefix = &encrypted_value[0..3];
        if prefix != b"v10" && prefix != b"v11" {
            return Err("Unsupported encryption version".into());
        }

        // IV (12バイト) と暗号文を分離
        let iv = &encrypted_value[3..15];
        let ciphertext = &encrypted_value[15..];

        // AES-256-GCM で復号化
        let cipher = Aes256Gcm::new_from_slice(key)?;
        let nonce = Nonce::from_slice(iv);

        let plaintext = cipher.decrypt(nonce, ciphertext)?;

        Ok(String::from_utf8(plaintext)?)
    }

    /// YouTube用のCookieを抽出
    pub fn extract_youtube_cookies() -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        let key = get_encryption_key()?;

        let local_appdata = std::env::var("LOCALAPPDATA")?;
        let db_path = PathBuf::from(&local_appdata)
            .join("Google")
            .join("Chrome")
            .join("User Data")
            .join("Default")
            .join("Network")
            .join("Cookies");

        // データベースをコピー（Chromeがロックしている可能性）
        let temp_path = std::env::temp_dir().join("liscov_chrome_cookies.sqlite");
        std::fs::copy(&db_path, &temp_path)?;

        let conn = Connection::open(&temp_path)?;

        let mut stmt = conn.prepare(
            "SELECT name, encrypted_value FROM cookies
             WHERE host_key LIKE '%youtube.com' OR host_key LIKE '%google.com'
             AND name IN ('SID', 'HSID', 'SSID', 'APISID', 'SAPISID')"
        )?;

        let mut cookies = HashMap::new();
        let mut rows = stmt.query([])?;

        while let Some(row) = rows.next()? {
            let name: String = row.get(0)?;
            let encrypted_value: Vec<u8> = row.get(1)?;

            if let Ok(value) = decrypt_cookie_value(&encrypted_value, &key) {
                cookies.insert(name, value);
            }
        }

        let _ = std::fs::remove_file(&temp_path);

        Ok(cookies)
    }
}
```

#### Python実装: browser_cookie3 ライブラリ

```python
# pip install browser-cookie3
import browser_cookie3

def extract_youtube_cookies_from_browser(browser: str = "chrome") -> dict:
    """ブラウザからYouTube Cookieを抽出"""

    required = ['SID', 'HSID', 'SSID', 'APISID', 'SAPISID']

    if browser == "chrome":
        cj = browser_cookie3.chrome(domain_name='.youtube.com')
    elif browser == "firefox":
        cj = browser_cookie3.firefox(domain_name='.youtube.com')
    elif browser == "edge":
        cj = browser_cookie3.edge(domain_name='.youtube.com')
    else:
        raise ValueError(f"Unsupported browser: {browser}")

    cookies = {}
    for cookie in cj:
        if cookie.name in required:
            cookies[cookie.name] = cookie.value

    return cookies

# 使用例
cookies = extract_youtube_cookies_from_browser("firefox")
print(f"Found cookies: {list(cookies.keys())}")
```

#### 利用可能なライブラリ

| 言語 | ライブラリ | Chrome | Firefox | Windows | macOS | Linux |
|------|-----------|--------|---------|---------|-------|-------|
| Python | [browser_cookie3](https://github.com/borisbabic/browser_cookie3) | ✓ | ✓ | ✓ | ✓ | ✓ |
| Rust | [browsercookie-rs](https://github.com/Ginkooo/browsercookie-rs) | ✗ | ✓ | ✗ | ✓ | ✓ |
| Rust | [extract-chrome-cookies](https://github.com/lei4519/extract-chrome-cookies) | ✓ | ✗ | ✗ | ✓ | ✓ |

#### 注意事項

1. **ブラウザを閉じる必要がある場合あり**
   - Chrome (Windows): データベースがロックされるため閉じる必要がある
   - Firefox: ファイルをコピーすれば開いたままでも可能

2. **セキュリティ考慮**
   - 復号化キーはOSの資格情報ストアに依存
   - 別ユーザーのCookieは復号化できない

3. **プロファイル指定**
   - 複数プロファイルがある場合は明示的に指定が必要

---

## SAPISIDHASH の生成

### アルゴリズム

```
SAPISIDHASH = {timestamp}_{hash}
hash = SHA1("{timestamp} {SAPISID} {origin}")
```

| パラメータ | 値 | 説明 |
|-----------|-----|------|
| `timestamp` | UNIXタイムスタンプ（秒） | `Math.floor(Date.now() / 1000)` |
| `SAPISID` | Cookie値 | `SAPISID` Cookieから取得 |
| `origin` | `https://www.youtube.com` | 固定値 |

### 生成フロー

```
1. 現在時刻をUNIXタイムスタンプ（秒）で取得
   timestamp = 1703836800

2. ハッシュ入力文字列を構築
   input = "1703836800 SAPISID値 https://www.youtube.com"

3. SHA-1ハッシュを計算
   hash = SHA1(input) = "a1b2c3d4e5f6..."

4. 結果を結合
   SAPISIDHASH = "1703836800_a1b2c3d4e5f6..."
```

### Rust実装

```rust
use sha1::{Sha1, Digest};
use std::time::{SystemTime, UNIX_EPOCH};

/// SAPISIDHASHを生成する
///
/// # Arguments
/// * `sapisid` - SAPISID Cookieの値
///
/// # Returns
/// * `String` - "timestamp_hash" 形式のSAPISIDHASH
pub fn generate_sapisidhash(sapisid: &str) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();

    let origin = "https://www.youtube.com";
    let input = format!("{} {} {}", timestamp, sapisid, origin);

    let mut hasher = Sha1::new();
    hasher.update(input.as_bytes());
    let hash = hasher.finalize();
    let hash_hex = hex::encode(hash);

    format!("{}_{}", timestamp, hash_hex)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sapisidhash_format() {
        let hash = generate_sapisidhash("test_sapisid");

        // 形式確認: timestamp_hexhash
        let parts: Vec<&str> = hash.split('_').collect();
        assert_eq!(parts.len(), 2);

        // タイムスタンプは数値
        assert!(parts[0].parse::<u64>().is_ok());

        // ハッシュは40文字の16進数（SHA-1）
        assert_eq!(parts[1].len(), 40);
        assert!(parts[1].chars().all(|c| c.is_ascii_hexdigit()));
    }
}
```

### JavaScript実装（参考）

```javascript
async function generateSapisidhash(sapisid) {
    const timestamp = Math.floor(Date.now() / 1000);
    const origin = "https://www.youtube.com";
    const input = `${timestamp} ${sapisid} ${origin}`;

    const encoder = new TextEncoder();
    const data = encoder.encode(input);
    const hashBuffer = await crypto.subtle.digest('SHA-1', data);
    const hashArray = Array.from(new Uint8Array(hashBuffer));
    const hashHex = hashArray.map(b => b.toString(16).padStart(2, '0')).join('');

    return `${timestamp}_${hashHex}`;
}
```

### Python実装（参考）

```python
import time
import hashlib

def generate_sapisidhash(sapisid: str) -> str:
    timestamp = int(time.time())
    origin = "https://www.youtube.com"

    hash_input = f"{timestamp} {sapisid} {origin}"
    hash_result = hashlib.sha1(hash_input.encode()).hexdigest()

    return f"{timestamp}_{hash_result}"
```

---

## HTTPリクエストヘッダー

### 必須ヘッダー

```http
POST https://www.youtube.com/youtubei/v1/live_chat/get_live_chat?key={API_KEY}

Authorization: SAPISIDHASH {sapisidhash}
Cookie: SID=xxx; HSID=xxx; SSID=xxx; APISID=xxx; SAPISID=xxx
X-Origin: https://www.youtube.com
Origin: https://www.youtube.com
Content-Type: application/json
```

### 追加推奨ヘッダー

```http
X-Goog-AuthUser: 0
X-YouTube-Client-Name: 1
X-YouTube-Client-Version: 2.20231219.04.00
User-Agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36
```

### ヘッダー詳細

| ヘッダー | 値 | 必須 | 説明 |
|----------|-----|------|------|
| `Authorization` | `SAPISIDHASH {値}` | ✓ | 認証トークン |
| `Cookie` | Cookie文字列 | ✓ | セッションCookie |
| `X-Origin` | `https://www.youtube.com` | ✓ | SAPISIDHASH検証用 |
| `Origin` | `https://www.youtube.com` | ✓ | CORS用 |
| `X-Goog-AuthUser` | `0` | △ | マルチアカウント時のインデックス |
| `X-Goog-Visitor-Id` | Base64値 | × | 訪問者ID（省略可） |

### X-Goog-AuthUser について

複数のGoogleアカウントにログインしている場合、どのアカウントを使用するか指定します：

```
https://myaccount.google.com/u/0/  → X-Goog-AuthUser: 0
https://myaccount.google.com/u/1/  → X-Goog-AuthUser: 1
https://myaccount.google.com/u/2/  → X-Goog-AuthUser: 2
```

---

## Cookieファイル形式

### Netscape Cookie形式（cookies.txt）

yt-dlp、curl、wget などで使用される標準形式です。

#### ファイル構造

```
# Netscape HTTP Cookie File
# https://curl.se/docs/http-cookies.html

.youtube.com	TRUE	/	TRUE	1735689600	SID	FgiOHj...
.youtube.com	TRUE	/	TRUE	1735689600	HSID	Aj8iK...
.youtube.com	TRUE	/	TRUE	1735689600	SSID	Bk9jL...
.youtube.com	TRUE	/	TRUE	1735689600	APISID	Cl0kM...
.youtube.com	TRUE	/	TRUE	1735689600	SAPISID	Dm1nO...
```

#### フィールド定義（7フィールド、TAB区切り）

| # | フィールド | 型 | 例 | 説明 |
|---|-----------|-----|-----|------|
| 0 | domain | String | `.youtube.com` | Cookieが有効なドメイン |
| 1 | subdomains | Boolean | `TRUE` | サブドメインでも有効か |
| 2 | path | String | `/` | Cookieが有効なパス |
| 3 | secure | Boolean | `TRUE` | HTTPS限定か |
| 4 | expiration | Number | `1735689600` | 有効期限（UNIXタイムスタンプ、0=セッション） |
| 5 | name | String | `SAPISID` | Cookie名 |
| 6 | value | String | `Dm1nO...` | Cookie値 |

#### Rust パーサー実装

```rust
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};

#[derive(Debug, Clone)]
pub struct Cookie {
    pub domain: String,
    pub include_subdomains: bool,
    pub path: String,
    pub secure: bool,
    pub expiration: u64,
    pub name: String,
    pub value: String,
}

pub struct CookieStore {
    cookies: HashMap<String, Cookie>,
}

impl CookieStore {
    /// Netscape形式のcookies.txtファイルを読み込む
    pub fn from_file(path: &str) -> Result<Self, std::io::Error> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut cookies = HashMap::new();

        for line in reader.lines() {
            let line = line?;

            // コメントと空行をスキップ
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }

            // HttpOnly対応
            let line = if line.starts_with("#HttpOnly_") {
                line.strip_prefix("#HttpOnly_").unwrap_or(&line)
            } else {
                &line
            };

            let fields: Vec<&str> = line.split('\t').collect();
            if fields.len() >= 7 {
                let cookie = Cookie {
                    domain: fields[0].to_string(),
                    include_subdomains: fields[1].eq_ignore_ascii_case("TRUE"),
                    path: fields[2].to_string(),
                    secure: fields[3].eq_ignore_ascii_case("TRUE"),
                    expiration: fields[4].parse().unwrap_or(0),
                    name: fields[5].to_string(),
                    value: fields[6].to_string(),
                };
                cookies.insert(cookie.name.clone(), cookie);
            }
        }

        Ok(Self { cookies })
    }

    /// 指定した名前のCookieを取得
    pub fn get(&self, name: &str) -> Option<&Cookie> {
        self.cookies.get(name)
    }

    /// 認証に必要なCookieがすべて揃っているか確認
    pub fn has_required_cookies(&self) -> bool {
        const REQUIRED: [&str; 5] = ["SID", "HSID", "SSID", "APISID", "SAPISID"];
        REQUIRED.iter().all(|name| self.cookies.contains_key(*name))
    }

    /// Cookie文字列を生成（HTTPヘッダー用）
    pub fn to_header_string(&self) -> String {
        self.cookies
            .values()
            .map(|c| format!("{}={}", c.name, c.value))
            .collect::<Vec<_>>()
            .join("; ")
    }

    /// 認証用Cookieのみの文字列を生成
    pub fn to_auth_header_string(&self) -> String {
        const AUTH_COOKIES: [&str; 5] = ["SID", "HSID", "SSID", "APISID", "SAPISID"];
        AUTH_COOKIES
            .iter()
            .filter_map(|name| self.cookies.get(*name))
            .map(|c| format!("{}={}", c.name, c.value))
            .collect::<Vec<_>>()
            .join("; ")
    }

    /// SAPISIDの値を取得
    pub fn get_sapisid(&self) -> Option<&str> {
        self.cookies.get("SAPISID").map(|c| c.value.as_str())
    }
}
```

---

## Rust実装例

### 認証付きInnerTubeクライアント

```rust
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, COOKIE, ORIGIN};
use sha1::{Sha1, Digest};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct AuthenticatedInnerTube {
    client: reqwest::Client,
    cookie_store: CookieStore,
    api_key: String,
}

impl AuthenticatedInnerTube {
    pub fn new(cookies_file: &str, api_key: String) -> Result<Self, Box<dyn std::error::Error>> {
        let cookie_store = CookieStore::from_file(cookies_file)?;

        if !cookie_store.has_required_cookies() {
            return Err("Missing required authentication cookies".into());
        }

        let client = reqwest::Client::builder()
            .default_headers(Self::build_default_headers())
            .build()?;

        Ok(Self {
            client,
            cookie_store,
            api_key,
        })
    }

    fn build_default_headers() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("X-Origin", HeaderValue::from_static("https://www.youtube.com"));
        headers.insert(ORIGIN, HeaderValue::from_static("https://www.youtube.com"));
        headers.insert("X-Goog-AuthUser", HeaderValue::from_static("0"));
        headers.insert("X-YouTube-Client-Name", HeaderValue::from_static("1"));
        headers
    }

    fn generate_sapisidhash(&self) -> Option<String> {
        let sapisid = self.cookie_store.get_sapisid()?;
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()?
            .as_secs();

        let origin = "https://www.youtube.com";
        let input = format!("{} {} {}", timestamp, sapisid, origin);

        let mut hasher = Sha1::new();
        hasher.update(input.as_bytes());
        let hash = hex::encode(hasher.finalize());

        Some(format!("{}_{}", timestamp, hash))
    }

    pub async fn get_live_chat(
        &self,
        continuation: &str,
    ) -> Result<serde_json::Value, reqwest::Error> {
        let url = format!(
            "https://www.youtube.com/youtubei/v1/live_chat/get_live_chat?key={}",
            self.api_key
        );

        let sapisidhash = self.generate_sapisidhash()
            .expect("Failed to generate SAPISIDHASH");

        let body = serde_json::json!({
            "context": {
                "client": {
                    "clientName": "WEB",
                    "clientVersion": "2.20231219.04.00"
                }
            },
            "continuation": continuation
        });

        let response = self.client
            .post(&url)
            .header(AUTHORIZATION, format!("SAPISIDHASH {}", sapisidhash))
            .header(COOKIE, self.cookie_store.to_auth_header_string())
            .json(&body)
            .send()
            .await?;

        response.json().await
    }
}
```

### 設定ファイル統合

```rust
// config.rs に追加

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// 認証を有効にするか
    pub enabled: bool,

    /// cookies.txtファイルのパス
    pub cookies_file: Option<String>,

    /// 複数アカウント時のインデックス（0始まり）
    pub auth_user: u32,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            cookies_file: None,
            auth_user: 0,
        }
    }
}
```

---

## トラブルシューティング

### よくあるエラー

#### 1. HTTP 401 Unauthorized

**原因**: 認証ヘッダーが無効

**確認事項**:
- SAPISIDHASHのタイムスタンプが現在時刻か
- originが `https://www.youtube.com` か
- Cookieが有効期限内か

#### 2. HTTP 403 Forbidden

**原因**: アクセス権限がない

**確認事項**:
- 対象チャンネルのメンバーシップを持っているか
- ログインアカウントが正しいか（X-Goog-AuthUser）

#### 3. Cookieが無効になった

**原因**: YouTubeがCookieをローテーション

**対策**:
- プライベートウィンドウでCookieを取得し直す
- ログアウトせずにウィンドウを閉じる

#### 4. Chrome からCookie抽出できない

**原因**: Chromeがデータベースをロック

**対策**:
```bash
# Chromeを完全に終了してから実行
chrome --disable-features=LockProfileCookieDatabase
```

### デバッグ方法

```rust
// 認証ヘッダーをログ出力（本番では無効化）
#[cfg(debug_assertions)]
fn debug_auth_headers(sapisidhash: &str, cookie: &str) {
    tracing::debug!(
        "Auth headers - SAPISIDHASH length: {}, Cookie length: {}",
        sapisidhash.len(),
        cookie.len()
    );
    // 注意: 実際の値はログに出力しない
}
```

---

## セキュリティ考慮事項

### やるべきこと

- [ ] Cookieファイルのアクセス権限を制限（`chmod 600`）
- [ ] Cookieをメモリ上で暗号化
- [ ] 不要になったCookieを確実に削除
- [ ] ログにCookie値を出力しない

### やってはいけないこと

- [ ] CookieをGitリポジトリにコミット
- [ ] Cookieを平文で設定ファイルに保存
- [ ] Cookieをエラーメッセージに含める
- [ ] SAPISIDHASHをキャッシュ（毎回生成すべき）

---

## 参考資料

- [YouTube.js Authentication Guide](https://ytjs.dev/guide/authentication)
- [SAPISIDHASH Generation (GitHub Gist)](https://gist.github.com/eyecatchup/2d700122e24154fdc985b7071ec7764a)
- [ytmusicapi Browser Authentication](https://ytmusicapi.readthedocs.io/en/stable/setup/browser.html)
- [Volumio-YouTube.js Cookie Guide](https://github.com/patrickkfkan/Volumio-YouTube.js/wiki/How-to-obtain-Cookie)
- [yt-dlp FAQ - Cookies](https://github.com/yt-dlp/yt-dlp/wiki/FAQ)
- [curl Cookie File Format](https://everything.curl.dev/http/cookies/fileformat.html)
- [Get cookies.txt LOCALLY (Chrome)](https://chromewebstore.google.com/detail/get-cookiestxt-locally/cclelndahbckbenkjhflpdbgdldlbecc)
- [cookies.txt (Firefox)](https://addons.mozilla.org/en-US/firefox/addon/cookies-txt/)
