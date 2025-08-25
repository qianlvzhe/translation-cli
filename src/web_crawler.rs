//! Web爬取模块 - 集成Monolith进行网页内容抓取和处理
//! 
//! 此模块负责：
//! - 使用Monolith库抓取完整的网页内容
//! - 将网页转换为独立的HTML文件（包含CSS、JS、图片等资源）
//! - 为后续的翻译处理准备标准化的HTML内容

// 标准库导入
use std::path::{Path, PathBuf};

// 第三方crate导入
use anyhow::{Context, Result};
use tracing::{debug, info, warn};

/// Web爬虫配置结构体
#[derive(Debug, Clone)]
pub struct WebCrawlerConfig {
    /// 目标URL
    pub url: String,
    /// 输出文件路径
    pub output_path: PathBuf,
    /// 是否包含CSS样式
    pub include_css: bool,
    /// 是否包含JavaScript
    pub include_js: bool,
    /// 是否包含图片资源
    pub include_images: bool,
    /// 用户代理字符串
    pub user_agent: String,
    /// 连接超时时间（秒）
    pub timeout: u64,
}

impl Default for WebCrawlerConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            output_path: PathBuf::new(),
            include_css: true,
            include_js: false, // 默认不包含JS，避免潜在的安全问题
            include_images: true,
            user_agent: "translation-cli/0.1.0 (Monolith Web Crawler)".to_string(),
            timeout: 30,
        }
    }
}

/// Web爬虫主要结构体
/// 
/// 集成Monolith库实现网页内容抓取和处理功能。
/// 支持自定义资源包含策略、重试机制和安全配置。
/// 
/// # Features
/// 
/// - 完整页面保存：将CSS、图片等资源内嵌为data URL
/// - 异步封装：使用tokio::spawn_blocking封装blocking API
/// - 重试机制：最多3次重试，指数退避延迟
/// - 资源配置：可选择性包含CSS、JS、图片等资源
pub struct WebCrawler {
    config: WebCrawlerConfig,
}

impl WebCrawler {
    /// 使用Monolith库进行实际的网页爬取
    async fn crawl_website(&self) -> Result<String> {
        let config = &self.config;
        
        // 创建monolith选项
        let mut options = monolith::core::Options {
            no_css: !config.include_css,
            no_js: !config.include_js,
            no_images: !config.include_images,
            user_agent: Some(config.user_agent.clone()),
            timeout: config.timeout,
            ignore_errors: false,
            silent: true, // 我们使用自己的日志系统
            ..Default::default()
        };

        debug!("Monolith选项: no_css={}, no_js={}, no_images={}, timeout={}s", 
            options.no_css, options.no_js, options.no_images, options.timeout);

        let target_url = config.url.clone();

        // 在blocking线程中执行monolith操作
        let result = tokio::task::spawn_blocking(move || {
            use monolith::core::create_monolithic_document;
            use monolith::cache::Cache;
            
            // 创建缓存，设置最小文件大小为0，不使用磁盘缓存文件
            let mut cache: Option<Cache> = Some(Cache::new(0, None));
            
            create_monolithic_document(target_url, &mut options, &mut cache)
        })
        .await
        .with_context(|| "Monolith任务执行失败")?;

        match result {
            Ok((html_bytes, title)) => {
                let html_content = String::from_utf8(html_bytes)
                    .with_context(|| "转换HTML字节为字符串失败")?;
                
                if let Some(page_title) = title {
                    info!("📄 网页标题: {}", page_title);
                }
                info!("✅ 网页内容爬取完成，大小: {} 字节", html_content.len());
                
                Ok(html_content)
            }
            Err(e) => {
                anyhow::bail!("Monolith爬取失败: {}", e);
            }
        }
    }

    /// 带重试机制的网页爬取
    async fn crawl_website_with_retry(&self) -> Result<String> {
        const MAX_RETRIES: u32 = 3;
        let mut last_error: Option<anyhow::Error> = None;

        for attempt in 1..=MAX_RETRIES {
            info!("🔄 尝试爬取网页 (第 {} 次)", attempt);
            
            match self.crawl_website().await {
                Ok(content) => {
                    if attempt > 1 {
                        info!("✅ 重试成功！");
                    }
                    return Ok(content);
                }
                Err(e) => {
                    warn!("❌ 爬取失败 (尝试 {}/{}): {}", attempt, MAX_RETRIES, e);
                    last_error = Some(e);
                    
                    if attempt < MAX_RETRIES {
                        let delay = std::time::Duration::from_secs(attempt as u64 * 2);
                        info!("⏳ 等待 {:?} 后重试...", delay);
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("所有重试尝试均失败")))
    }
    /// 创建新的Web爬虫实例
    pub fn new(config: WebCrawlerConfig) -> Self {
        Self { config }
    }

    /// 使用默认配置创建Web爬虫
    pub fn with_url(url: &str) -> Self {
        let mut config = WebCrawlerConfig::default();
        config.url = url.to_string();
        Self::new(config)
    }

    /// 设置输出路径
    pub fn output_to<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.config.output_path = path.as_ref().to_path_buf();
        self
    }

    /// 配置资源包含选项
    pub fn include_resources(mut self, css: bool, js: bool, images: bool) -> Self {
        self.config.include_css = css;
        self.config.include_js = js;
        self.config.include_images = images;
        self
    }

    /// 设置用户代理
    pub fn user_agent(mut self, user_agent: &str) -> Self {
        self.config.user_agent = user_agent.to_string();
        self
    }

    /// 设置连接超时
    pub fn timeout(mut self, seconds: u64) -> Self {
        self.config.timeout = seconds;
        self
    }

    /// 执行网页爬取
    /// 
    /// 返回爬取的HTML内容字符串和输出文件的路径
    pub async fn crawl(&self) -> Result<(String, PathBuf)> {
        info!("🕷️ 开始爬取网页: {}", self.config.url);
        debug!("爬虫配置: {:?}", self.config);

        // 验证URL
        self.validate_url()?;

        // 准备输出路径
        let output_path = self.prepare_output_path()?;
        debug!("输出路径: {}", output_path.display());

        // 使用重试机制爬取网页
        let html_content = self.crawl_website_with_retry().await
            .with_context(|| format!("爬取网页失败: {}", self.config.url))?;

        // 写入到输出文件（如果指定了输出路径）
        if self.config.output_path != PathBuf::new() {
            std::fs::write(&output_path, &html_content)
                .with_context(|| format!("写入输出文件失败: {}", output_path.display()))?;
            info!("✅ 网页已保存到: {}", output_path.display());
        }

        Ok((html_content, output_path))
    }

    /// 验证URL格式
    fn validate_url(&self) -> Result<()> {
        if self.config.url.is_empty() {
            anyhow::bail!("URL不能为空");
        }

        if !self.config.url.starts_with("http://") && !self.config.url.starts_with("https://") {
            anyhow::bail!("URL必须以http://或https://开头");
        }

        Ok(())
    }

    /// 准备输出路径
    fn prepare_output_path(&self) -> Result<PathBuf> {
        let output_path = if self.config.output_path == PathBuf::new() {
            // 如果没有指定输出路径，根据URL生成默认路径
            self.generate_default_output_path()?
        } else {
            self.config.output_path.clone()
        };

        // 确保输出目录存在
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("创建输出目录失败: {}", parent.display()))?;
        }

        Ok(output_path)
    }

    /// 根据URL生成默认输出文件名
    fn generate_default_output_path(&self) -> Result<PathBuf> {
        use url::Url;
        
        let parsed_url = Url::parse(&self.config.url)
            .with_context(|| format!("解析URL失败: {}", self.config.url))?;
        
        let host = parsed_url.host_str().unwrap_or("unknown");
        let path = parsed_url.path();
        
        // 生成安全的文件名
        let mut filename = if path == "/" || path.is_empty() {
            format!("{}_index", host)
        } else {
            format!("{}{}", host, path.replace('/', "_"))
        };
        
        // 清理文件名中的非法字符
        filename = filename
            .replace(['<', '>', ':', '"', '|', '?', '*'], "_")
            .replace("__", "_")
            .trim_matches('_')
            .to_string();
        
        // 添加时间戳避免冲突
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let output_filename = format!("{}_{}.html", filename, timestamp);
        Ok(std::env::current_dir()?.join(output_filename))
    }
}

/// 便捷函数：快速爬取网页到指定路径
pub async fn crawl_url_to_file<P: AsRef<Path>>(
    url: &str,
    output_path: P,
) -> Result<String> {
    let crawler = WebCrawler::with_url(url).output_to(output_path);
    let (content, _) = crawler.crawl().await?;
    Ok(content)
}

/// 便捷函数：爬取网页并返回HTML内容（不保存到文件）
pub async fn crawl_url_to_string(url: &str) -> Result<String> {
    let temp_path = std::env::temp_dir().join("temp_crawl.html");
    crawl_url_to_file(url, &temp_path).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_crawler_config_default() {
        let config = WebCrawlerConfig::default();
        assert!(config.url.is_empty());
        assert_eq!(config.include_css, true);
        assert_eq!(config.include_js, false);
        assert_eq!(config.include_images, true);
        assert_eq!(config.timeout, 30);
        assert_eq!(config.user_agent, "translation-cli/0.1.0 (Monolith Web Crawler)");
    }

    #[test]
    fn test_web_crawler_builder() {
        let crawler = WebCrawler::with_url("https://example.com")
            .output_to("output.html")
            .include_resources(true, true, false)
            .user_agent("test-agent")
            .timeout(60);

        assert_eq!(crawler.config.url, "https://example.com");
        assert_eq!(crawler.config.output_path, PathBuf::from("output.html"));
        assert_eq!(crawler.config.include_css, true);
        assert_eq!(crawler.config.include_js, true);
        assert_eq!(crawler.config.include_images, false);
        assert_eq!(crawler.config.user_agent, "test-agent");
        assert_eq!(crawler.config.timeout, 60);
    }

    #[test]
    fn test_url_validation() {
        let crawler = WebCrawler::with_url("");
        assert!(crawler.validate_url().is_err());

        let crawler = WebCrawler::with_url("ftp://example.com");
        assert!(crawler.validate_url().is_err());

        let crawler = WebCrawler::with_url("https://example.com");
        assert!(crawler.validate_url().is_ok());

        let crawler = WebCrawler::with_url("http://example.com");
        assert!(crawler.validate_url().is_ok());
    }

    #[test]
    fn test_generate_default_output_path() {
        let crawler = WebCrawler::with_url("https://example.com/path/to/page");
        let path = crawler.generate_default_output_path().unwrap();
        
        let filename = path.file_name().unwrap().to_string_lossy();
        assert!(filename.contains("example.com"));
        assert!(filename.contains("path_to_page"));
        assert!(filename.ends_with(".html"));
    }

    #[test]
    fn test_generate_default_output_path_root() {
        let crawler = WebCrawler::with_url("https://example.com/");
        let path = crawler.generate_default_output_path().unwrap();
        
        let filename = path.file_name().unwrap().to_string_lossy();
        assert!(filename.contains("example.com_index"));
        assert!(filename.ends_with(".html"));
    }

    #[test]
    fn test_filename_sanitization() {
        let crawler = WebCrawler::with_url("https://example.com/path:with<bad>chars?query=1");
        let path = crawler.generate_default_output_path().unwrap();
        
        let filename = path.file_name().unwrap().to_string_lossy();
        // 确保没有非法字符
        assert!(!filename.contains('<'));
        assert!(!filename.contains('>'));
        assert!(!filename.contains(':'));
        assert!(!filename.contains('?'));
    }

    #[tokio::test]
    async fn test_crawl_invalid_url() {
        let crawler = WebCrawler::with_url("invalid-url");
        let result = crawler.crawl().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_crawl_nonexistent_domain() {
        let crawler = WebCrawler::with_url("https://this-domain-should-not-exist-12345.com")
            .timeout(5); // 短超时避免测试时间过长
        
        let result = crawler.crawl().await;
        // 应该失败，但我们不检查具体错误类型，因为可能因网络环境而异
        assert!(result.is_err());
    }

    // 模拟测试 - 测试爬虫配置和基本功能
    #[test]
    fn test_crawl_workflow_components() {
        // 测试URL验证
        let valid_urls = vec![
            "https://example.com",
            "http://test.org",
            "https://subdomain.example.com/path",
        ];
        
        for url in valid_urls {
            let crawler = WebCrawler::with_url(url);
            assert!(crawler.validate_url().is_ok(), "URL should be valid: {}", url);
        }

        // 测试无效URL
        let invalid_urls = vec![
            "",
            "ftp://example.com",
            "example.com",
        ];
        
        for url in invalid_urls {
            let crawler = WebCrawler::with_url(url);
            assert!(crawler.validate_url().is_err(), "URL should be invalid: {}", url);
        }
    }

    #[test]
    fn test_output_path_preparation() {
        use std::env;
        
        // 测试默认输出路径生成
        let crawler = WebCrawler::with_url("https://example.com");
        let path = crawler.prepare_output_path().unwrap();
        assert!(path.is_absolute());
        assert!(path.to_string_lossy().contains("example.com"));
        
        // 测试指定输出路径
        let temp_dir = env::temp_dir();
        let output_path = temp_dir.join("test_output.html");
        let crawler = WebCrawler::with_url("https://example.com")
            .output_to(&output_path);
        
        let prepared_path = crawler.prepare_output_path().unwrap();
        assert_eq!(prepared_path, output_path);
    }
}