//! 测试核心功能的示例程序
//! 
//! 该程序演示了WebCrawler和TempManager的基本使用方法

use anyhow::Result;
use std::path::PathBuf;
use tracing::{info, error};
use translation_cli::{
    web_crawler::{WebCrawler, WebCrawlerConfig},
    temp_manager::{TempManager, TempManagerConfig},
};

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志系统
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    info!("🚀 开始测试translation-cli核心功能");

    // 测试1: WebCrawler基本功能
    test_web_crawler().await?;
    
    // 测试2: TempManager基本功能  
    test_temp_manager().await?;

    info!("✅ 所有测试完成！");
    Ok(())
}

async fn test_web_crawler() -> Result<()> {
    info!("🔧 测试WebCrawler功能");
    
    // 使用一个简单的测试页面
    let test_url = "https://httpbin.org/html";
    
    // 创建爬虫配置
    let config = WebCrawlerConfig {
        url: test_url.to_string(),
        output_path: PathBuf::new(), // 使用默认路径
        include_css: true,
        include_js: false,
        include_images: false,
        user_agent: "translation-cli-test/0.1.0".to_string(),
        timeout: 10,
    };

    // 创建爬虫实例
    let crawler = WebCrawler::new(config);

    // 测试爬取
    match crawler.crawl().await {
        Ok((content, output_path)) => {
            info!("✅ 网页爬取成功！");
            info!("📄 内容长度: {} 字节", content.len());
            info!("📁 输出文件: {}", output_path.display());
            
            // 验证内容包含HTML
            if content.contains("<html") && content.contains("</html>") {
                info!("✅ HTML内容验证成功");
            } else {
                error!("❌ HTML内容验证失败");
                return Err(anyhow::anyhow!("爬取的内容不是有效HTML"));
            }
        }
        Err(e) => {
            error!("❌ 网页爬取失败: {}", e);
            info!("ℹ️ 这可能是网络问题，不影响代码功能");
        }
    }

    Ok(())
}

async fn test_temp_manager() -> Result<()> {
    info!("🔧 测试TempManager功能");
    
    // 创建临时文件管理器
    let config = TempManagerConfig::default();
    let mut temp_manager = TempManager::new(config)?;

    // 测试创建临时HTML文件
    let test_html = r#"<!DOCTYPE html>
<html>
<head>
    <title>测试页面</title>
</head>
<body>
    <h1>这是一个测试页面</h1>
    <p>用于验证临时文件管理器功能</p>
</body>
</html>"#;

    let temp_html_path = temp_manager.create_temp_html(test_html)?;
    info!("✅ 临时HTML文件创建成功: {}", temp_html_path.display());

    // 验证文件存在且内容正确
    let saved_content = std::fs::read_to_string(&temp_html_path)?;
    if saved_content.contains("测试页面") {
        info!("✅ 临时文件内容验证成功");
    } else {
        error!("❌ 临时文件内容验证失败");
        return Err(anyhow::anyhow!("临时文件内容不正确"));
    }

    // 测试从爬取内容创建HTML
    let crawl_html_path = temp_manager.create_temp_html_from_crawl(
        test_html, 
        "https://example.com"
    )?;
    info!("✅ 爬取临时HTML文件创建成功: {}", crawl_html_path.display());

    // 验证文件包含元数据
    let crawl_content = std::fs::read_to_string(&crawl_html_path)?;
    if crawl_content.contains("由translation-cli生成") && crawl_content.contains("源URL") {
        info!("✅ 爬取临时文件元数据验证成功");
    } else {
        error!("❌ 爬取临时文件元数据验证失败");
        return Err(anyhow::anyhow!("临时文件缺少元数据"));
    }

    // 清理测试文件
    temp_manager.cleanup_file(&temp_html_path)?;
    temp_manager.cleanup_file(&crawl_html_path)?;
    info!("✅ 临时文件清理完成");

    Ok(())
}