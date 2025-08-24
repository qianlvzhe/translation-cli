/// 翻译API配置常量
/// 
/// 该文件定义了所有翻译服务相关的常量配置，方便统一管理和维护

/// 默认翻译API配置
pub mod api_config {
    /// 默认翻译API地址
    pub const DEFAULT_API_URL: &str = "https://deepl3.fileaiwork.online/dptrans?token=ej0ab47388ed86e843de9f499e52e6e664ae1m491cad7bf1.bIrYaAAAAAA=.b9c326068ac3c37ff36b8fea77867db51ddf235150945d7ad43472d68581e6c4pd14&newllm=1";
    
    /// 本地开发API地址
    pub const LOCAL_API_URL: &str = "http://localhost:1188/translate";
    
    /// 备用API地址列表
    pub const BACKUP_API_URLS: &[&str] = &[
        "https://api.deepl.com/v2/translate",
        "https://translate.googleapis.com/translate_a/single",
        "http://localhost:8080/translate",
    ];
}

/// 翻译服务配置
pub mod service_config {
    /// 默认目标语言
    pub const DEFAULT_TARGET_LANG: &str = "zh";
    
    /// 支持的语言代码
    pub const SUPPORTED_LANGUAGES: &[&str] = &[
        "zh", "en", "ja", "ko", "fr", "de", "es", "it", "pt", "ru",
        "ar", "hi", "th", "vi", "id", "ms", "tl", "nl", "sv", "da",
        "no", "fi", "pl", "cs", "sk", "hu", "ro", "bg", "hr", "sr",
        "sl", "et", "lv", "lt", "mt", "ga", "cy", "is", "mk", "sq"
    ];
    
    /// 默认批处理大小
    pub const DEFAULT_BATCH_SIZE: usize = 25;
    
    /// 大文件批处理大小
    pub const LARGE_BATCH_SIZE: usize = 100;
    
    /// 默认最大重试次数
    pub const DEFAULT_MAX_RETRIES: usize = 3;
    
    /// 默认并发批次数量
    pub const DEFAULT_CONCURRENT_BATCHES: usize = 5;
    
    /// 请求超时时间（秒）
    pub const REQUEST_TIMEOUT_SECONDS: u64 = 30;
}

/// 网页爬取配置
pub mod crawler_config {
    /// 默认爬取超时时间（秒）
    pub const DEFAULT_CRAWL_TIMEOUT: u64 = 30;
    
    /// 默认User-Agent
    pub const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (compatible; TranslationCLI/0.2.0; +https://github.com/translation-cli)";
    
    /// 最大重试次数
    pub const MAX_CRAWL_RETRIES: usize = 3;
    
    /// 重试延迟基数（毫秒）
    pub const RETRY_DELAY_BASE_MS: u64 = 1000;
    
    /// 最大页面大小（字节）
    pub const MAX_PAGE_SIZE_BYTES: usize = 50 * 1024 * 1024; // 50MB
}

/// 错误消息常量
pub mod error_messages {
    /// 网络连接错误
    pub const NETWORK_ERROR: &str = "网络连接失败，请检查网络连接或API地址";
    
    /// API认证错误
    pub const AUTH_ERROR: &str = "API认证失败，请检查API密钥或令牌";
    
    /// 不支持的语言错误
    pub const UNSUPPORTED_LANGUAGE: &str = "不支持的目标语言";
    
    /// 文件读取错误
    pub const FILE_READ_ERROR: &str = "无法读取输入文件";
    
    /// 文件写入错误
    pub const FILE_WRITE_ERROR: &str = "无法写入输出文件";
    
    /// HTML解析错误
    pub const HTML_PARSE_ERROR: &str = "HTML内容解析失败";
}

/// 性能相关配置
pub mod performance_config {
    /// 亚秒级性能目标（毫秒）
    pub const SUB_SECOND_TARGET_MS: u128 = 1000;
    
    /// 内存使用警告阈值（字节）
    pub const MEMORY_WARNING_THRESHOLD_BYTES: usize = 100 * 1024 * 1024; // 100MB
    
    /// 最大并发连接数
    pub const MAX_CONCURRENT_CONNECTIONS: usize = 10;
}

/// 实用工具函数
/// 获取API URL，根据本地模式标志选择
pub fn get_api_url(local_api: bool, custom_api: Option<&str>) -> &str {
    if let Some(custom) = custom_api {
        if !custom.is_empty() && custom != "****" {
            return custom;
        }
    }
    
    if local_api {
        api_config::LOCAL_API_URL
    } else {
        api_config::DEFAULT_API_URL
    }
}

/// 验证API URL是否有效
pub fn is_valid_api_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

/// 验证语言代码是否支持
pub fn is_supported_language(lang: &str) -> bool {
    service_config::SUPPORTED_LANGUAGES.contains(&lang)
}

/// 获取批处理大小
pub fn get_batch_size(large_batch: bool, custom_size: Option<usize>) -> usize {
    if let Some(size) = custom_size {
        size
    } else if large_batch {
        service_config::LARGE_BATCH_SIZE
    } else {
        service_config::DEFAULT_BATCH_SIZE
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_api_url_selection() {
        assert_eq!(get_api_url(true, None), api_config::LOCAL_API_URL);
        assert_eq!(get_api_url(false, None), api_config::DEFAULT_API_URL);
        assert_eq!(get_api_url(false, Some("http://custom.api")), "http://custom.api");
    }
    
    #[test]
    fn test_language_validation() {
        assert!(is_supported_language("zh"));
        assert!(is_supported_language("en"));
        assert!(!is_supported_language("xx"));
    }
    
    #[test]
    fn test_batch_size_selection() {
        assert_eq!(get_batch_size(false, None), service_config::DEFAULT_BATCH_SIZE);
        assert_eq!(get_batch_size(true, None), service_config::LARGE_BATCH_SIZE);
        assert_eq!(get_batch_size(false, Some(50)), 50);
    }
    
    #[test]
    fn test_api_url_validation() {
        assert!(is_valid_api_url("https://example.com"));
        assert!(is_valid_api_url("http://localhost:8080"));
        assert!(!is_valid_api_url("ftp://example.com"));
        assert!(!is_valid_api_url("invalid-url"));
    }
}