# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Communication Language
**基本的に日本語でやり取りを行ってください。** コード中のコメントも日本語で記述してください。ただし、日本語では表現が困難な技術的概念については英語を使用して構いません。

## Development Commands

**Build & Run:**
```bash
# Main GUI application
cargo run --bin liscov

# Test data generator 
cargo run --bin generate_test_data

# Build with debug features
cargo run --features debug-tokio --bin liscov
cargo run --features debug-full --bin liscov

# Release build with debug symbols
cargo build --profile release-with-debug
```

**Testing:**
```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture
```

**Platform Features:**
```bash
# Desktop (default)
cargo run --features desktop --bin liscov

# Web compilation
cargo build --features web
```

## Architecture Overview

### State Management Pattern
The application uses **event-driven architecture** with a global `StateManager`:
- `AppEvent` enum defines all state changes
- `StateManager` is a singleton (OnceLock) with tokio mpsc channels
- Dioxus components subscribe to state via signals
- All mutations flow through: `User Input → AppEvent → StateManager → Signal Updates → UI Re-render`

### GUI Architecture (Dioxus 0.6.3)
- **MainWindow**: Root component with integrated header/tab layout (migrated from separate header/tab components)
- **TabContent**: Dynamic content switching based on `ActiveTab` enum
- **Custom Hooks**: `use_live_chat()` connects components to services
- **Window State**: Automatic saving/restoration via `ConfigManager` with 1-second polling

### YouTube API Integration
- **InnerTube API**: Custom implementation bypassing official API
- **Stateful Continuation**: Live chat pagination using continuation tokens
- **Error Hierarchy**: `FetchError` → `YoutubeFetchError` → `LiveChatError`
- **Data Flow**: `YouTube Live Chat → InnerTube API → ResponseEntry → GuiChatMessage → UI`

### Database Schema
Core entities with session-centric design:
- `sessions` (streaming sessions) 
- `messages` (chat messages)
- `viewer_profiles` (audience analytics)
- `questions` (detected Q&A)
- `hourly_revenue` & `contributor_stats` (monetization)

All relationships cascade from sessions. Schema uses SQLite with JSON metadata columns and optimized indexing on session_id/timestamp.

### Analytics Pipeline
Modular components:
- `revenue_analyzer`: Super Chat/membership tracking
- `engagement_tracker`: Viewer interaction metrics
- `trend_analyzer`: Temporal patterns
- `data_exporter`: Multi-format export (CSV/Excel/JSON) with streaming for large datasets

### Configuration Management
Multi-layer configuration system:
- `AppConfig`: Application settings using XDG directories
- `WindowConfig`: Persistent window state (auto-saved on exit)
- `debug.toml`: Development debugging configuration
- `Dioxus.toml`: Framework settings

### I/O Processing
- **NDJSON**: Multiple parsing strategies (legacy, timestamped, generic) with error recovery
- **Raw Response Saving**: Configurable file rotation with size limits
- **Atomic Operations**: Prevents file corruption during concurrent access

## Development Context

**Technology Migration**: Currently migrating from Slint to Dioxus 0.6.3. The codebase represents Phase 0-1 of this migration focusing on technical validation and basic structure.

**Resource Management**: 
- 1000 message display limit to prevent memory issues
- Background cleanup and file rotation
- Careful async task lifecycle management

**Debugging Infrastructure**:
- Structured logging with tracing crate
- Optional tokio-console integration via `debug-tokio` feature
- Development-specific logging configuration in `debug.toml`
