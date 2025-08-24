use anyhow::{Context, Result};
use clap::Parser;
use std::time::Instant;
use tracing::{error, info, warn};

mod config;
mod stats;
mod utils;
mod html_processor;
mod translator;

use config::{Cli, LocalTranslationConfig, LocalTranslationStats};
use stats::{TranslationStats, print_performance_stats, format_duration};
use utils::{init_logging, validate_input_file, generate_output_path};
use translator::translate_with_indexed_mode;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // 初始化日志系统
    init_logging(cli.verbose, cli.quiet);

    // 验证输入文件
    validate_input_file(&cli.input)?;

    // 生成输出文件路径
    let output_path = generate_output_path(&cli.input, &cli.output, &cli.lang);

    if !cli.quiet {
        info!("🚀 启动HTML翻译 - 目标: 亚秒级性能");
        info!("📂 输入文件: {}", cli.input.display());
        info!("📄 输出文件: {}", output_path.display());
        info!("🌐 目标语言: {}", cli.lang);
    }

    // 开始性能计时
    let total_start = Instant::now();

    // 执行翻译
    match translate_file(&cli, &output_path).await {
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

/// 翻译文件核心函数（重构版：去除html-translation-lib依赖）
async fn translate_file(cli: &Cli, output_path: &std::path::PathBuf) -> Result<TranslationStats> {
    let config_start = Instant::now();

    // 动态优化配置
    let api_url = if cli.local_api {
        "http://localhost:1188/translate"
    } else {
        &cli.api
    };

    let batch_size = if cli.large_batch {
        100 // 大文件使用更大批次
    } else {
        cli.batch_size
    };

    // 创建本地配置（替代TranslationConfig）
    let _config = LocalTranslationConfig::new()
        .target_language(&cli.lang)
        .api_url(api_url)
        .enable_cache(!cli.no_cache)
        .batch_size(batch_size)
        .max_retries(cli.max_retries);

    let config_duration = config_start.elapsed();

    // 读取文件
    let read_start = Instant::now();
    let html_content = std::fs::read_to_string(&cli.input)
        .with_context(|| format!("读取文件失败: {}", cli.input.display()))?;
    let read_duration = read_start.elapsed();

    if cli.verbose {
        info!("📏 文件大小: {} 字节", html_content.len());
        info!("🚀 使用内置索引标记翻译 - 高性能模式");
        info!("🔀 并发批次数量: {}", cli.concurrent_batches);
    }

    // 使用内置高性能索引翻译（完全独立实现）
    let translate_start = Instant::now();
    let translated_content = translate_with_indexed_mode(&html_content, api_url, cli.concurrent_batches, cli.verbose)
        .await?;
    let translate_duration = translate_start.elapsed();

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
    })
}