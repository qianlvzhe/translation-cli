//! Translation CLI - 高性能HTML翻译工具库
//! 
//! 这个库提供了网页爬取、HTML处理、文本翻译和临时文件管理等核心功能。

pub mod web_crawler;
pub mod temp_manager;
pub mod translator;
pub mod utils;
pub mod html_processor;
pub mod error;
pub mod config;
pub mod stats;
pub mod api_constants;

// 导出核心类型
pub use error::{TranslationError, Result};
pub use config::{LocalTranslationConfig, Cli};
pub use utils::InputSource;