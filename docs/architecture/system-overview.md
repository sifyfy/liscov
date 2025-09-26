# liscov システム全体アーキテクチャ

## 📊 システム概要

liscovは4つの主要レイヤーから構成される階層化アーキテクチャを採用しています：

1. **Presentation Layer** (GUI - Dioxus 0.6.3)
2. **State Management Layer** (イベント駆動状態管理)
3. **Service Layer** (ビジネスロジック・API統合)
4. **Data Layer** (永続化・外部API)

## 🏗️ 全体アーキテクチャ図

```mermaid
graph TB
    %% Entry Point
    Entry[liscov.rs<br/>🚀 アプリケーション<br/>エントリポイント]
    
    %% GUI Layer
    subgraph GUI["🖥️ GUI Layer (Dioxus 0.6.3)"]
        MainWindow[MainWindow<br/>メインウィンドウ]
        ChatDisplay[ChatDisplay<br/>チャット表示]
        TabNav[TabNavigation<br/>タブナビゲーション]
        Revenue[RevenueDashboard<br/>収益ダッシュボード]
        Export[ExportPanel<br/>エクスポートパネル]
        Filter[FilterPanel<br/>フィルターパネル]
        Input[InputSection<br/>入力セクション]
        Status[StatusPanel<br/>ステータスパネル]
    end
    
    %% State Management Layer
    subgraph State["🔄 State Management Layer"]
        StateManager[StateManager<br/>状態管理]
        EventBus[EventBus<br/>イベントバス]
        Signals[Signal System<br/>Dioxusシグナル]
        AppState[AppState<br/>アプリケーション状態]
    end
    
    %% Service Layer
    subgraph Services["⚙️ Service Layer"]
        LiveChatSvc[LiveChatService<br/>ライブチャットサービス]
        AnalyticsSvc[AnalyticsService<br/>分析サービス]
        ChatMgmt[ChatManagement<br/>チャット管理]
        PluginSys[PluginSystem<br/>プラグインシステム]
        ConfigMgr[ConfigManager<br/>設定管理]
    end
    
    %% Data Layer
    subgraph Data["💾 Data Layer"]
        InnerTube[YouTube InnerTube<br/>🌐 API]
        SQLite[SQLite Database<br/>🗄️ ローカルDB]
        FileIO[File I/O System<br/>📁 ファイル処理]
        RawSaver[Raw Response Saver<br/>📄 生データ保存]
    end
    
    %% Connections
    Entry --> MainWindow
    
    MainWindow --> ChatDisplay
    MainWindow --> TabNav
    TabNav --> Revenue
    TabNav --> Export
    TabNav --> Filter
    MainWindow --> Input
    MainWindow --> Status
    
    ChatDisplay --> StateManager
    Revenue --> StateManager
    Export --> StateManager
    Filter --> StateManager
    Input --> StateManager
    Status --> StateManager
    
    StateManager --> EventBus
    StateManager --> AppState
    EventBus --> Signals
    Signals --> ChatDisplay
    Signals --> Revenue
    Signals --> Status
    
    StateManager --> LiveChatSvc
    StateManager --> AnalyticsSvc
    StateManager --> ChatMgmt
    
    LiveChatSvc --> ConfigMgr
    AnalyticsSvc --> PluginSys
    
    LiveChatSvc --> InnerTube
    AnalyticsSvc --> SQLite
    ChatMgmt --> SQLite
    ConfigMgr --> FileIO
    LiveChatSvc --> RawSaver
    
    %% Styling
    classDef entryPoint fill:#ff6b6b,stroke:#d63447,stroke-width:3px,color:#fff
    classDef guiLayer fill:#4ecdc4,stroke:#26d0ce,stroke-width:2px,color:#fff
    classDef stateLayer fill:#45b7d1,stroke:#3742fa,stroke-width:2px,color:#fff
    classDef serviceLayer fill:#f9ca24,stroke:#f0932b,stroke-width:2px,color:#000
    classDef dataLayer fill:#6c5ce7,stroke:#5f3dc4,stroke-width:2px,color:#fff
    
    class Entry entryPoint
    class MainWindow,ChatDisplay,TabNav,Revenue,Export,Filter,Input,Status guiLayer
    class StateManager,EventBus,Signals,AppState stateLayer
    class LiveChatSvc,AnalyticsSvc,ChatMgmt,PluginSys,ConfigMgr serviceLayer
    class InnerTube,SQLite,FileIO,RawSaver dataLayer
```

## 🔄 データフロー図

```mermaid
sequenceDiagram
    participant User
    participant GUI
    participant State
    participant Service
    participant Data
    
    Note over User,Data: ライブチャット接続フロー
    
    User->>GUI: YouTubeURLを入力
    GUI->>State: CurrentUrlUpdated
    State->>Service: URL変更通知
    Service->>Data: InnerTube APIリクエスト
    Data-->>Service: 初期チャットデータ
    Service->>State: MessageAdded
    State->>GUI: Signal更新
    GUI-->>User: チャット表示
    
    Note over User,Data: リアルタイムメッセージ受信
    
    loop 継続ポーリング
        Service->>Data: 継続トークンでリクエスト
        Data-->>Service: 新規メッセージ
        Service->>State: MessagesAdded
        State->>GUI: Signal更新
        GUI-->>User: 新規メッセージ表示
    end
    
    Note over User,Data: 分析・エクスポート
    
    User->>GUI: エクスポート要求
    GUI->>State: ExportRequested
    State->>Service: 分析処理開始
    Service->>Data: データベースクエリ
    Data-->>Service: 集計データ
    Service->>Data: ファイル保存
    Data-->>Service: 保存完了
    Service->>State: ExportCompleted
    State->>GUI: 完了通知
    GUI-->>User: 成功メッセージ
```

## 🧩 モジュール相互関係

```mermaid
graph LR
    subgraph "src/"
        bin[bin/<br/>🚀 エントリポイント]
        gui[gui/<br/>🖥️ GUI層]
        api[api/<br/>🌐 API統合]
        db[database/<br/>🗄️ データベース]
        analytics[analytics/<br/>📊 分析]
        chat[chat_management/<br/>💬 チャット管理]
        io[io/<br/>📁 I/O処理]
    end
    
    bin --> gui
    gui --> api
    gui --> db
    gui --> analytics
    gui --> chat
    gui --> io
    
    analytics --> db
    chat --> db
    api --> io
    
    classDef entry fill:#ff6b6b,stroke:#d63447,stroke-width:2px,color:#fff
    classDef gui fill:#4ecdc4,stroke:#26d0ce,stroke-width:2px,color:#fff
    classDef service fill:#f9ca24,stroke:#f0932b,stroke-width:2px,color:#000
    classDef data fill:#6c5ce7,stroke:#5f3dc4,stroke-width:2px,color:#fff
    
    class bin entry
    class gui gui
    class api,analytics,chat service
    class db,io data
```

## 🔌 イベント駆動アーキテクチャ

```mermaid
graph TD
    subgraph EventSystem["🔄 イベントシステム"]
        Events[AppEvent<br/>📨 システムイベント]
        StateManager[StateManager<br/>🎯 状態管理]
        Subscribers[Event Subscribers<br/>👂 イベント購読者]
    end
    
    subgraph EventTypes["📋 イベント種別"]
        MessageAdded[MessageAdded<br/>💬 メッセージ追加]
        ConnectionChanged[ConnectionChanged<br/>🔗 接続状態変更]
        ServiceStateChanged[ServiceStateChanged<br/>⚙️ サービス状態変更]
        StatsUpdated[StatsUpdated<br/>📊 統計更新]
        ExportRequested[ExportRequested<br/>📤 エクスポート要求]
    end
    
    subgraph EventFlow["🌊 イベントフロー"]
        Producer[Event Producer<br/>📤 イベント発行者]
        Channel[mpsc::channel<br/>📡 非同期チャネル]
        Processor[Event Processor<br/>⚡ イベント処理]
        Consumer[Event Consumer<br/>📥 イベント消費者]
    end
    
    Producer --> Channel
    Channel --> Processor
    Processor --> Consumer
    
    Events --> StateManager
    StateManager --> Subscribers
    
    MessageAdded --> Events
    ConnectionChanged --> Events
    ServiceStateChanged --> Events
    StatsUpdated --> Events
    ExportRequested --> Events
    
    classDef event fill:#45b7d1,stroke:#3742fa,stroke-width:2px,color:#fff
    classDef flow fill:#f9ca24,stroke:#f0932b,stroke-width:2px,color:#000
    
    class Events,StateManager,Subscribers event
    class Producer,Channel,Processor,Consumer flow
```

## 🚀 アプリケーション起動フロー

```mermaid
graph TD
    Start([アプリケーション開始]) --> ParseArgs[CLI引数解析]
    ParseArgs --> LoadConfig[設定ファイル読み込み]
    LoadConfig --> InitLogging[ログシステム初期化]
    InitLogging --> ValidateWindow[ウィンドウ設定検証]
    ValidateWindow --> InitPlugins[プラグインシステム初期化]
    InitPlugins --> SetupSignal[シグナルハンドラー設定]
    SetupSignal --> CreateWindow[Dioxusウィンドウ作成]
    CreateWindow --> StartEventLoop[イベントループ開始]
    StartEventLoop --> StartStateManager[状態管理開始]
    StartStateManager --> InitServices[サービス初期化]
    InitServices --> ShowGUI[GUI表示]
    ShowGUI --> Ready([アプリケーション準備完了])
    
    %% エラーハンドリング
    LoadConfig -->|失敗| DefaultConfig[デフォルト設定使用]
    DefaultConfig --> InitLogging
    
    classDef start fill:#ff6b6b,stroke:#d63447,stroke-width:2px,color:#fff
    classDef process fill:#4ecdc4,stroke:#26d0ce,stroke-width:2px,color:#fff
    classDef ready fill:#6c5ce7,stroke:#5f3dc4,stroke-width:2px,color:#fff
    
    class Start,Ready start
    class ParseArgs,LoadConfig,InitLogging,ValidateWindow,InitPlugins,SetupSignal,CreateWindow,StartEventLoop,StartStateManager,InitServices,ShowGUI process
    class DefaultConfig ready
```

## 📈 パフォーマンス考慮事項

### メモリ管理戦略

- **循環バッファ**: 最大1000メッセージのメモリ制限
- **バッチ処理**: 大量メッセージの効率的処理
- **レイジーローディング**: 必要時のみデータロード

### 非同期処理最適化

- **Tokio Runtime**: マルチスレッド非同期実行
- **Channel-based Communication**: コンポーネント間通信
- **背景タスク**: UI阻害なしのデータ処理

### レスポンシブネス

- **Dioxus Signals**: リアクティブUI更新
- **イベント駆動**: 効率的状態変更通知
- **プログレッシブレンダリング**: 段階的UI描画

---

**最終更新**: 2025-06-25  
**対象バージョン**: 0.1.0  
**アーキテクチャレベル**: System Overview
