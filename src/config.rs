//! 配置管理模块
//!
//! 提供CLI参数解析和翻译配置管理功能

// 标准库导入
use std::path::PathBuf;

// 第三方crate导入
use clap::Parser;

// 本地模块导入
use crate::api_constants::api_config;

/// 本地翻译配置结构体
/// 
/// 用于替代html-translation-lib中的TranslationConfig，提供更轻量级的配置管理。
/// 支持Builder模式进行链式配置。
/// 
/// # Examples
/// 
/// ```rust
/// use translation_cli::config::LocalTranslationConfig;
/// 
/// let config = LocalTranslationConfig::new()
///     .target_language("zh")
///     .api_url("http://localhost:1188/translate")
///     .batch_size(50)
///     .max_retries(5);
/// ```
#[derive(Debug, Clone)]
pub struct LocalTranslationConfig {
    /// 目标语言代码 (如: zh, en, ja, ko)
    target_lang: String,
    /// 翻译API服务地址
    api_url: String,
    /// 批处理大小
    batch_size: usize,
    /// 最大重试次数
    max_retries: usize,
    /// 是否启用缓存
    enable_cache: bool,
}

impl LocalTranslationConfig {
    /// 创建新的配置实例
    /// 
    /// 返回具有默认值的配置实例：
    /// - 目标语言: 中文 ("zh")
    /// - API地址: 本地开发服务器
    /// - 批处理大小: 25
    /// - 最大重试次数: 3
    /// - 启用缓存: true
    pub fn new() -> Self {
        Self {
            target_lang: "zh".to_string(),
            api_url: "http://localhost:1188/translate".to_string(),
            batch_size: 25,
            max_retries: 3,
            enable_cache: true,
        }
    }
    
    /// 获取目标语言代码
    pub fn target_lang(&self) -> &str {
        &self.target_lang
    }
    
    /// 获取API地址
    pub fn api_url(&self) -> &str {
        &self.api_url
    }
    
    /// 获取批处理大小
    pub fn batch_size(&self) -> usize {
        self.batch_size
    }
    
    /// 获取最大重试次数
    pub fn max_retries(&self) -> usize {
        self.max_retries
    }
    
    /// 检查是否启用缓存
    pub fn is_cache_enabled(&self) -> bool {
        self.enable_cache
    }
    
    /// 设置目标语言代码
    pub fn target_language(mut self, lang: &str) -> Self {
        self.target_lang = lang.to_string();
        self
    }
    
    /// 设置API地址
    pub fn with_api_url(mut self, url: &str) -> Self {
        self.api_url = url.to_string();
        self
    }
    
    /// 设置批处理大小
    pub fn with_batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }
    
    /// 设置最大重试次数
    pub fn with_max_retries(mut self, retries: usize) -> Self {
        self.max_retries = retries;
        self
    }
    
    /// 设置是否启用缓存
    pub fn enable_cache(mut self, enable: bool) -> Self {
        self.enable_cache = enable;
        self
    }
}

impl Default for LocalTranslationConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// CLI参数结构
#[derive(Parser)]
#[command(author, version, about = "高性能HTML翻译CLI工具 - 支持亚秒级文件翻译和网页爬取翻译", long_about = None)]
pub struct Cli {
    /// 输入HTML文件路径或网页URL
    #[arg(short, long, value_name = "FILE_OR_URL")]
    pub input: String,

    /// 输出文件路径 (可选，默认为输入文件名+语言代码)
    #[arg(short, long, value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// 目标语言代码 (如: zh, en, ja, ko)
    #[arg(short, long, default_value = "zh")]
    pub lang: String,

    /// 翻译API地址
    #[arg(short, long, default_value = api_config::DEFAULT_API_URL)]
    pub api: String,

    /// 批处理大小 (优化性能)
    #[arg(long, default_value = "25")]
    pub batch_size: usize,

    /// 最大重试次数
    #[arg(long, default_value = "3")]
    pub max_retries: usize,

    /// 禁用缓存
    #[arg(long)]
    pub no_cache: bool,

    /// 详细输出模式
    #[arg(short, long)]
    pub verbose: bool,

    /// 静默模式 (仅输出错误)
    #[arg(short, long)]
    pub quiet: bool,

    /// 显示性能统计
    #[arg(long)]
    pub stats: bool,

    /// 增大批处理大小 (用于大文件优化)
    #[arg(long)]
    pub large_batch: bool,

    /// 使用本地API (localhost:1188)
    #[arg(long)]
    pub local_api: bool,

    /// 并发批次数量 (默认5)
    #[arg(long, default_value = "5")]
    pub concurrent_batches: usize,

    /// 启用网页爬取模式
    #[arg(long, help = "从URL爬取网页内容进行翻译")]
    pub from_url: bool,

    /// 保留临时HTML文件用于调试
    #[arg(long, help = "保留爬取的临时HTML文件用于调试分析")]
    pub keep_temp: bool,

    /// 爬取时包含图片资源
    #[arg(long, help = "爬取网页时包含图片资源")]
    pub include_images: bool,

    /// 爬取时包含CSS样式
    #[arg(long, help = "爬取网页时包含CSS样式文件")]
    pub include_css: bool,

    /// 爬取时包含JavaScript
    #[arg(long, help = "爬取网页时包含JavaScript文件")]
    pub include_js: bool,

    /// 爬取超时时间（秒）
    #[arg(long, default_value = "30", help = "网页爬取的超时时间（秒）")]
    pub crawl_timeout: u64,

    /// 自定义User-Agent
    #[arg(long, help = "自定义User-Agent字符串")]
    pub user_agent: Option<String>,
}

/// 本地翻译统计结构（简化版本）
#[derive(Debug, Default)]
pub struct LocalTranslationStats {
    pub texts_collected: usize,
    pub texts_filtered: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub batches_created: usize,
}