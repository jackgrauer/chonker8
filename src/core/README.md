# Core Module

Core application logic and infrastructure.

## Files:

- **app.rs** - Main application state and lifecycle (if we create this)

- **config.rs** - Configuration management
  - UI settings
  - File paths
  - User preferences

- **hot_reload.rs** - File watching and auto-rebuild
  - Watches source files for changes
  - Automatically rebuilds when files change
  - This is why it's called "chonker8-hot"