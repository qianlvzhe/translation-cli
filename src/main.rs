//! Translation CLI 主程序入口
//!
//! 高性能HTML翻译命令行工具，支持文件和URL两种输入模式

// 标准库导入
use std::time::Instant;

// 第三方crate导入
use anyhow::{Context, Result};
use clap::Parser;
use tracing::{error, info, warn};

// 本地模块导入
mod api_constants;
mod config;
mod error;
mod stats;
mod utils;
mod html_processor;
mod translator;
mod web_crawler;
mod temp_manager;

use config::{Cli, LocalTranslationConfig, LocalTranslationStats};
use error::TranslationError;
use stats::{TranslationStats, print_performance_stats, format_duration};
use utils::{init_logging, validate_input_source, generate_output_path_for_source, InputSource};
use translator::translate_with_indexed_mode;
use web_crawler::WebCrawler;
use temp_manager::TempManager;
use api_constants::{get_api_url, get_batch_size};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // 初始化日志系统
    init_logging(cli.verbose, cli.quiet);

    // 验证输入源（文件或URL）
    let input_source = validate_input_source(&cli.input)?;

    // 生成输出文件路径
    let output_path = generate_output_path_for_source(&input_source, &cli.output, &cli.lang);

    if !cli.quiet {
        info!("🚀 启动HTML翻译 - 目标: 亚秒级性能");
        info!("📂 输入源: {}", match &input_source {
            InputSource::File(path) => format!("📁 文件: {}", path.display()),
            InputSource::Url(url) => format!("🌐 URL: {}", url),
        });
        info!("📄 输出文件: {}", output_path.display());
        info!("🌐 目标语言: {}", cli.lang);
    }

    // 开始性能计时
    let total_start = Instant::now();

    // 执行翻译
    match translate_source(&cli, &input_source, &output_path).await {
        Ok(stats) => {
            let total_duration = total_start.elapsed();

            if !cli.quiet {
                info!("✅ 翻译完成！总耗时: {:.3}秒", total_duration.as_secs_f64());
            }

            // 显示性能统计
            if cli.stats || cli.verbose {
                print_performance_stats(&stats, total_duration);
            }

            // 检查是否达到亚秒级性能目标
            if total_duration.as_millis() < 1000 {
                if !cli.quiet {
                    info!(
                        "🎯 性能目标达成: {} < 1000ms",
                        format_duration(total_duration)
                    );
                }
            } else {
                warn!("⚠️  未达到亚秒级目标: {}", format_duration(total_duration));
            }
        }
        Err(e) => {
            error!("❌ 翻译失败: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}

/// 根据输入源类型分发翻译任务的主路由函数
async fn translate_source(cli: &Cli, input_source: &InputSource, output_path: &std::path::PathBuf) -> Result<TranslationStats> {
    match input_source {
        InputSource::File(file_path) => {
            if !cli.quiet {
                info!("📁 开始文件翻译模式");
            }
            translate_from_file(cli, file_path, output_path).await
        },
        InputSource::Url(url) => {
            if !cli.quiet {
                info!("🌐 开始URL翻译模式");
            }
            translate_from_url(cli, url, output_path).await
        }
    }
}
/// 处理本地文件翻译的核心函数
async fn translate_from_file(cli: &Cli, file_path: &std::path::PathBuf, output_path: &std::path::PathBuf) -> Result<TranslationStats> {
    let config_start = Instant::now();

    // 动态优化配置，使用API常量
    let api_url = get_api_url(cli.local_api, Some(&cli.api));
    let batch_size = get_batch_size(cli.large_batch, Some(cli.batch_size));

    // 创建本地配置（替代TranslationConfig）
    let _config = LocalTranslationConfig::new()
        .target_language(&cli.lang)
        .with_api_url(api_url)
        .enable_cache(!cli.no_cache)
        .with_batch_size(batch_size)
        .with_max_retries(cli.max_retries);

    let config_duration = config_start.elapsed();

    // 读取文件内容
    let read_start = Instant::now();
    let html_content = std::fs::read_to_string(file_path)
        .with_context(|| format!("读取文件失败: {}", file_path.display()))?;
    let read_duration = read_start.elapsed();

    if cli.verbose {
        info!("🔧 翻译配置初始化完成，耗时: {:.3}秒", config_duration.as_secs_f64());
        info!("📖 文件读取完成，耗时: {:.3}秒", read_duration.as_secs_f64());
        info!("📏 文件大小: {} 字节", html_content.len());
        info!("🚀 使用内置索引标记翻译 - 高性能模式");
        info!("🔀 并发批次数量: {}", cli.concurrent_batches);
    }

    // 使用内置高性能索引翻译（完全独立实现）
    let translate_start = Instant::now();
    let translated_content = translate_with_indexed_mode(&html_content, api_url, cli.concurrent_batches, cli.verbose)
        .await?;
    let translate_duration = translate_start.elapsed();

    if cli.verbose {
        info!("🔤 翻译处理完成，耗时: {:.3}秒", translate_duration.as_secs_f64());
        info!("📊 翻译结果大小: {} 字节", translated_content.len());
    }

    // 创建本地统计信息
    let local_stats = LocalTranslationStats {
        texts_collected: 0, // 这些统计信息在索引翻译模式中不直接适用
        texts_filtered: 0,
        cache_hits: 0,
        cache_misses: 0,
        batches_created: cli.concurrent_batches,
    };

    // 写入文件
    let write_start = Instant::now();
    std::fs::write(output_path, &translated_content)
        .with_context(|| format!("写入文件失败: {}", output_path.display()))?;
    let write_duration = write_start.elapsed();

    if cli.verbose {
        info!("💾 文件写入完成，耗时: {:.3}秒", write_duration.as_secs_f64());
        info!("✅ 翻译文件已保存: {}", output_path.display());
    }

    Ok(TranslationStats {
        config_time: config_duration,
        translator_init_time: std::time::Duration::from_millis(0), // 无需初始化翻译器
        file_read_time: read_duration,
        translation_time: translate_duration,
        file_write_time: write_duration,
        input_size: html_content.len(),
        output_size: translated_content.len(),
        texts_collected: local_stats.texts_collected,
        texts_filtered: local_stats.texts_filtered,
        cache_hits: local_stats.cache_hits,
        cache_misses: local_stats.cache_misses,
        batches_created: local_stats.batches_created,
        // 文件翻译不涉及网页爬取
        crawl_time: std::time::Duration::from_millis(0),
        crawl_retries: 0,
        temp_file_size: 0,
        final_url: None,
    })
}

/// 处理URL翻译的主流程函数
/// 集成WebCrawler、TempManager和翻译引擎的完整流程
async fn translate_from_url(cli: &Cli, url: &url::Url, output_path: &std::path::PathBuf) -> Result<TranslationStats> {
    let config_start = Instant::now();

    // 动态优化配置，使用API常量
    let api_url = get_api_url(cli.local_api, Some(&cli.api));
    let batch_size = get_batch_size(cli.large_batch, Some(cli.batch_size));

    // 创建本地配置
    let _config = LocalTranslationConfig::new()
        .target_language(&cli.lang)
        .with_api_url(api_url)
        .enable_cache(!cli.no_cache)
        .with_batch_size(batch_size)
        .with_max_retries(cli.max_retries);

    let config_duration = config_start.elapsed();

    if cli.verbose {
        info!("🔧 翻译配置初始化完成，耗时: {:.3}秒", config_duration.as_secs_f64());
    }

    // 创建临时文件管理器
    let mut temp_manager = TempManager::default()
        .with_context(|| "创建临时文件管理器失败")?;
    
    if cli.verbose {
        info!("📁 临时文件管理器已创建");
    }

    // 使用WebCrawler爬取网页
    let crawl_start = Instant::now();
    
    let web_crawler = WebCrawler::with_url(url.as_str())
        .include_resources(true, false, true) // 包含CSS和图片，不包含JS避免安全问题
        .timeout(30);

    let (html_content, _temp_path) = web_crawler.crawl().await
        .with_context(|| format!("网页爬取失败: {}", url))?;
    
    let crawl_duration = crawl_start.elapsed();

    if cli.verbose {
        info!("🕷️ 网页爬取完成，耗时: {:.3}秒", crawl_duration.as_secs_f64());
        info!("📏 网页内容大小: {} 字节", html_content.len());
        info!("🚀 使用内置索引标记翻译 - 高性能模式");
        info!("🔀 并发批次数量: {}", cli.concurrent_batches);
    }

    // 创建临时HTML文件用于翻译处理
    let temp_html_path = temp_manager.create_temp_html_from_crawl(&html_content, url.as_str())
        .with_context(|| "创建临时HTML文件失败")?;

    if cli.verbose {
        info!("📝 临时HTML文件: {}", temp_html_path.display());
    }

    // 使用内置高性能索引翻译
    let translate_start = Instant::now();
    let translated_content = translate_with_indexed_mode(&html_content, api_url, cli.concurrent_batches, cli.verbose)
        .await
        .with_context(|| "翻译处理失败")?;
    let translate_duration = translate_start.elapsed();

    if cli.verbose {
        info!("🔤 翻译处理完成，耗时: {:.3}秒", translate_duration.as_secs_f64());
        info!("📊 翻译结果大小: {} 字节", translated_content.len());
    }

    // 创建本地统计信息
    let local_stats = LocalTranslationStats {
        texts_collected: 0, // 这些统计信息在索引翻译模式中不直接适用
        texts_filtered: 0,
        cache_hits: 0,
        cache_misses: 0,
        batches_created: cli.concurrent_batches,
    };

    // 写入最终文件
    let write_start = Instant::now();
    std::fs::write(output_path, &translated_content)
        .with_context(|| format!("写入输出文件失败: {}", output_path.display()))?;
    let write_duration = write_start.elapsed();

    if cli.verbose {
        info!("💾 文件写入完成，耗时: {:.3}秒", write_duration.as_secs_f64());
        info!("✅ 翻译文件已保存: {}", output_path.display());
    }

    // 临时文件会在TempManager被drop时自动清理
    if cli.verbose {
        info!("🧹 临时文件将在程序结束时自动清理");
    }

    Ok(TranslationStats {
        config_time: config_duration,
        translator_init_time: std::time::Duration::from_millis(0), // 无需初始化翻译器
        file_read_time: crawl_duration, // 将爬取时间作为读取时间
        translation_time: translate_duration,
        file_write_time: write_duration,
        input_size: html_content.len(),
        output_size: translated_content.len(),
        texts_collected: local_stats.texts_collected,
        texts_filtered: local_stats.texts_filtered,
        cache_hits: local_stats.cache_hits,
        cache_misses: local_stats.cache_misses,
        batches_created: local_stats.batches_created,
        // 网页爬取相关统计
        crawl_time: crawl_duration,
        crawl_retries: 0, // TODO: 从WebCrawler获取重试次数
        temp_file_size: html_content.len(),
        final_url: Some(url.to_string()),
    })
}