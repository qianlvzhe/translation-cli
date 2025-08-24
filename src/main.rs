use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use clap::Parser;
use html_translation_lib::{HtmlTranslator, TranslationConfig};
use markup5ever_rcdom::RcDom;
use regex::Regex;
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Instant;
use tracing::{error, info, warn};

#[derive(Parser)]
#[command(author, version, about = "高性能HTML翻译CLI工具 - 支持亚秒级文件翻译", long_about = None)]
struct Cli {
    /// 输入HTML文件的绝对路径
    #[arg(short, long, value_name = "FILE")]
    input: PathBuf,

    /// 输出文件路径 (可选，默认为输入文件名+语言代码)
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,

    /// 目标语言代码 (如: zh, en, ja, ko)
    #[arg(short, long, default_value = "zh")]
    lang: String,

    /// 翻译API地址
    #[arg(short, long, default_value = "****")]
    api: String,

    /// 批处理大小 (优化性能)
    #[arg(long, default_value = "25")]
    batch_size: usize,

    /// 最大重试次数
    #[arg(long, default_value = "3")]
    max_retries: usize,

    /// 禁用缓存
    #[arg(long)]
    no_cache: bool,

    /// 详细输出模式
    #[arg(short, long)]
    verbose: bool,

    /// 静默模式 (仅输出错误)
    #[arg(short, long)]
    quiet: bool,

    /// 显示性能统计
    #[arg(long)]
    stats: bool,

    /// 增大批处理大小 (用于大文件优化)
    #[arg(long)]
    large_batch: bool,

    /// 使用本地API (localhost:1188)
    #[arg(long)]
    local_api: bool,

    /// 启用索引标记翻译 (大幅提升性能)
    #[arg(long)]
    indexed_translation: bool,

    /// 并发批次数量 (默认5)
    #[arg(long, default_value = "5")]
    concurrent_batches: usize,
}

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

/// 翻译文件核心函数
async fn translate_file(cli: &Cli, output_path: &PathBuf) -> Result<TranslationStats> {
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

    // 创建优化配置
    let config = TranslationConfig::new()
        .target_language(&cli.lang)
        .api_url(api_url)
        .enable_cache(!cli.no_cache)
        .batch_size(batch_size)
        .max_retries(cli.max_retries);

    let config_duration = config_start.elapsed();

    // 创建翻译器
    let translator_start = Instant::now();
    let mut translator = HtmlTranslator::new(config)
        .await
        .context("创建翻译器失败")?;
    let translator_duration = translator_start.elapsed();

    // 读取文件
    let read_start = Instant::now();
    let html_content = std::fs::read_to_string(&cli.input)
        .with_context(|| format!("读取文件失败: {}", cli.input.display()))?;
    let read_duration = read_start.elapsed();

    if cli.verbose {
        info!("📏 文件大小: {} 字节", html_content.len());
        if cli.indexed_translation {
            info!("🚀 启用索引标记翻译 - 大幅提升性能");
            info!("🔀 并发批次数量: {}", cli.concurrent_batches);
        }
    }

    // 执行翻译 - 选择翻译策略
    let translate_start = Instant::now();
    let translated_content = if cli.indexed_translation {
        // 使用高性能索引翻译
        translate_with_indexed_mode(&html_content, api_url, cli.concurrent_batches, cli.verbose)
            .await?
    } else {
        // 使用原始的依赖库翻译
        translator
            .translate_html(&html_content)
            .await
            .context("HTML翻译失败")?
    };
    let translate_duration = translate_start.elapsed();

    // 获取翻译统计
    let lib_stats = translator.get_stats();

    // 写入文件
    let write_start = Instant::now();
    std::fs::write(output_path, &translated_content)
        .with_context(|| format!("写入文件失败: {}", output_path.display()))?;
    let write_duration = write_start.elapsed();

    Ok(TranslationStats {
        config_time: config_duration,
        translator_init_time: translator_duration,
        file_read_time: read_duration,
        translation_time: translate_duration,
        file_write_time: write_duration,
        input_size: html_content.len(),
        output_size: translated_content.len(),
        texts_collected: lib_stats.texts_collected,
        texts_filtered: lib_stats.texts_filtered,
        cache_hits: lib_stats.cache_hits,
        cache_misses: lib_stats.cache_misses,
        batches_created: lib_stats.batches_created,
    })
}

/// 自定义统计结构
#[derive(Debug)]
struct TranslationStats {
    config_time: std::time::Duration,
    translator_init_time: std::time::Duration,
    file_read_time: std::time::Duration,
    translation_time: std::time::Duration,
    file_write_time: std::time::Duration,
    input_size: usize,
    output_size: usize,
    texts_collected: usize,
    texts_filtered: usize,
    cache_hits: usize,
    cache_misses: usize,
    batches_created: usize,
}

/// 初始化日志系统
fn init_logging(verbose: bool, quiet: bool) {
    if quiet {
        return;
    }

    let level = if verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };

    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .init();
}

/// 验证输入文件
fn validate_input_file(path: &PathBuf) -> Result<()> {
    if !path.exists() {
        anyhow::bail!("输入文件不存在: {}", path.display());
    }

    if !path.is_file() {
        anyhow::bail!("输入路径不是文件: {}", path.display());
    }

    if let Some(ext) = path.extension() {
        if ext != "html" && ext != "htm" {
            warn!("⚠️  文件扩展名不是HTML: {}", ext.to_string_lossy());
        }
    }

    Ok(())
}

/// 生成输出文件路径
fn generate_output_path(input: &PathBuf, output: &Option<PathBuf>, lang: &str) -> PathBuf {
    if let Some(output_path) = output {
        return output_path.clone();
    }

    // 自动生成输出路径: input_zh.html
    let stem = input.file_stem().unwrap_or_default();
    let extension = input.extension().unwrap_or_default();

    let output_name = format!(
        "{}_{}.{}",
        stem.to_string_lossy(),
        lang,
        extension.to_string_lossy()
    );

    if let Some(parent) = input.parent() {
        parent.join(output_name)
    } else {
        PathBuf::from(output_name)
    }
}

/// 打印性能统计
fn print_performance_stats(stats: &TranslationStats, total_duration: std::time::Duration) {
    println!("\n📊 性能统计报告:");
    println!("═══════════════════════════════════════");

    // 时间分解
    println!("⏱️  时间分解:");
    println!("   配置创建: {}", format_duration(stats.config_time));
    println!(
        "   翻译器初始化: {}",
        format_duration(stats.translator_init_time)
    );
    println!("   文件读取: {}", format_duration(stats.file_read_time));
    println!("   翻译执行: {}", format_duration(stats.translation_time));
    println!("   文件写入: {}", format_duration(stats.file_write_time));
    println!("   总耗时: {}", format_duration(total_duration));

    // 文件统计
    println!("\n📏 文件统计:");
    println!(
        "   输入大小: {} 字节 ({:.1} KB)",
        stats.input_size,
        stats.input_size as f64 / 1024.0
    );
    println!(
        "   输出大小: {} 字节 ({:.1} KB)",
        stats.output_size,
        stats.output_size as f64 / 1024.0
    );
    println!(
        "   大小变化: {:.1}%",
        (stats.output_size as f64 / stats.input_size as f64 - 1.0) * 100.0
    );

    // 翻译统计
    println!("\n🔤 翻译统计:");
    println!("   收集文本: {} 项", stats.texts_collected);
    println!("   过滤后文本: {} 项", stats.texts_filtered);
    println!("   创建批次: {} 个", stats.batches_created);

    // 缓存统计
    if stats.cache_hits + stats.cache_misses > 0 {
        let cache_hit_rate =
            stats.cache_hits as f64 / (stats.cache_hits + stats.cache_misses) as f64;
        println!("\n💾 缓存统计:");
        println!("   缓存命中: {} 次", stats.cache_hits);
        println!("   缓存未命中: {} 次", stats.cache_misses);
        println!("   命中率: {:.1}%", cache_hit_rate * 100.0);
    }

    // 性能指标
    println!("\n🚀 性能指标:");
    println!(
        "   处理速度: {:.1} KB/s",
        stats.input_size as f64 / 1024.0 / total_duration.as_secs_f64()
    );

    let performance_grade = match total_duration.as_millis() {
        0..=500 => "🏆 优秀",
        501..=800 => "👍 良好",
        801..=1000 => "✅ 达标",
        _ => "⚠️  需优化",
    };
    println!("   性能评级: {}", performance_grade);
}

/// 格式化持续时间
fn format_duration(duration: std::time::Duration) -> String {
    let millis = duration.as_millis();
    if millis < 1000 {
        format!("{}ms", millis)
    } else {
        format!("{:.3}s", duration.as_secs_f64())
    }
}

/// HTML性能预处理 - 智能过滤不需要翻译的内容
fn preprocess_html_for_performance(html: &str, essential_only: bool) -> String {
    if essential_only {
        // 仅翻译关键内容模式 - 使用DOM解析确保结构完整
        extract_essential_content_safely(html)
    } else {
        // 快速模式 - 只做安全的优化
        optimize_html_safely(html)
    }
}

/// 安全地优化HTML（不破坏结构）
fn optimize_html_safely(html: &str) -> String {
    use regex::Regex;

    let mut processed = html.to_string();

    // 1. 只移除完整的样式块和脚本块（确保标签匹配）
    let style_regex = Regex::new(r"(?s)<style[^>]*>.*?</style>").unwrap();
    let script_regex = Regex::new(r"(?s)<script[^>]*>.*?</script>").unwrap();

    processed = style_regex.replace_all(&processed, "").to_string();
    processed = script_regex.replace_all(&processed, "").to_string();

    // 2. 移除HTML注释（安全操作）
    let comment_regex = Regex::new(r"(?s)<!--.*?-->").unwrap();
    processed = comment_regex.replace_all(&processed, "").to_string();

    // 3. 压缩空白字符（但保留结构）
    let whitespace_regex = Regex::new(r"\s{2,}").unwrap();
    processed = whitespace_regex.replace_all(&processed, " ").to_string();

    // 4. 移除长的Base64数据URLs（图片数据），但保留标签结构
    let base64_regex = Regex::new(r#"(data:[^;]+;base64,)[A-Za-z0-9+/=]{200,}"#).unwrap();
    processed = base64_regex
        .replace_all(&processed, "${1}[removed]")
        .to_string();

    processed
}

/// 安全地提取关键内容（保持DOM结构）
fn extract_essential_content_safely(html: &str) -> String {
    // 在这种模式下，我们创建一个最小化的HTML结构
    // 只包含真正需要翻译的内容标签

    use regex::Regex;

    // 先进行安全优化
    let optimized = optimize_html_safely(html);

    // 提取标题、段落等关键文本内容
    let title_regex = Regex::new(r"(?s)<title[^>]*>(.*?)</title>").unwrap();
    let h_regex = Regex::new(r"(?s)<h[1-6][^>]*>(.*?)</h[1-6]>").unwrap();
    let p_regex = Regex::new(r"(?s)<p[^>]*>(.*?)</p>").unwrap();
    let li_regex = Regex::new(r"(?s)<li[^>]*>(.*?)</li>").unwrap();

    let mut essential_content = Vec::new();

    // 提取标题
    if let Some(title_match) = title_regex.find(&optimized) {
        essential_content.push(format!(
            "<title>{}</title>",
            title_regex
                .captures(&optimized)
                .and_then(|cap| cap.get(1))
                .map(|m| m.as_str())
                .unwrap_or("")
        ));
    }

    // 提取所有标题标签
    for mat in h_regex.find_iter(&optimized) {
        essential_content.push(mat.as_str().to_string());
    }

    // 提取段落（限制数量避免过多）
    let mut p_count = 0;
    for mat in p_regex.find_iter(&optimized) {
        if p_count < 50 {
            // 限制段落数量
            essential_content.push(mat.as_str().to_string());
            p_count += 1;
        }
    }

    // 提取列表项（限制数量）
    let mut li_count = 0;
    for mat in li_regex.find_iter(&optimized) {
        if li_count < 30 {
            // 限制列表项数量
            essential_content.push(format!(
                "<p>{}</p>",
                li_regex
                    .captures(mat.as_str())
                    .and_then(|cap| cap.get(1))
                    .map(|m| m.as_str())
                    .unwrap_or("")
            ));
            li_count += 1;
        }
    }

    // 如果提取到内容，创建简化的HTML结构
    if !essential_content.is_empty() {
        format!(
            "<!DOCTYPE html><html><head><meta charset=\"UTF-8\"></head><body>{}</body></html>",
            essential_content.join("")
        )
    } else {
        // 如果没有提取到内容，返回安全优化的版本
        optimized
    }
}

/// 使用索引模式的高性能翻译
async fn translate_with_indexed_mode(
    html_content: &str,
    api_url: &str,
    concurrent_batches: usize,
    verbose: bool,
) -> Result<String> {
    use html5ever::parse_document;
    use html5ever::tendril::TendrilSink;
    use markup5ever_rcdom::RcDom;

    // 1. 解析HTML
    let dom = parse_document(RcDom::default(), Default::default())
        .from_utf8()
        .read_from(&mut html_content.as_bytes())
        .map_err(|e| anyhow::anyhow!("HTML解析失败: {:?}", e))?;

    // 2. 提取所有可翻译文本
    let texts = extract_translatable_texts(&dom);

    if verbose {
        info!("📝 提取到 {} 个可翻译文本", texts.len());
    }

    if texts.is_empty() {
        return Ok(html_content.to_string());
    }

    // 3. 使用索引标记批量翻译
    let text_strings: Vec<String> = texts.iter().map(|t| t.clone()).collect();
    let translations =
        indexed_batch_translation(text_strings, api_url, concurrent_batches, verbose).await?;

    // 4. 应用翻译结果到DOM
    let translated_dom = apply_translations_to_dom(dom, &texts, &translations)?;

    // 5. 序列化为HTML
    serialize_dom_to_html(translated_dom)
}

/// 提取DOM中的可翻译文本
fn extract_translatable_texts(dom: &RcDom) -> Vec<String> {
    use markup5ever_rcdom::NodeData;
    use regex::Regex;
    use std::collections::{HashSet, VecDeque};

    let mut texts = Vec::new();
    let mut seen_texts = HashSet::new(); // 防止重复
    let mut queue = VecDeque::new();
    queue.push_back(dom.document.clone());

    // 用于匹配JavaScript字符串的正则表达式
    let js_string_regex = Regex::new(r#"(?:['"`])([^'"`]{3,})(?:['"`])"#).unwrap();
    // 用于匹配JSON字符串的正则表达式
    let json_string_regex = Regex::new(r#""([^"]{3,})"\s*:"#).unwrap();

    while let Some(node) = queue.pop_front() {
        match node.data {
            NodeData::Text { ref contents } => {
                let text = contents.borrow().to_string();
                let trimmed = text.trim();
                // 更宽松的文本过滤条件
                if trimmed.len() > 1
                    && !trimmed
                        .chars()
                        .all(|c| c.is_whitespace() || c.is_ascii_punctuation())
                    && !seen_texts.contains(trimmed)
                {
                    texts.push(trimmed.to_string());
                    seen_texts.insert(trimmed.to_string());
                }
            }
            NodeData::Element {
                ref name,
                ref attrs,
                ..
            } => {
                let tag_name = name.local.as_ref();

                // 检查可翻译属性
                for attr in attrs.borrow().iter() {
                    let attr_name = attr.name.local.as_ref();
                    let attr_value = attr.value.trim();

                    // 标准可翻译属性
                    if matches!(attr_name, "title" | "alt" | "placeholder") {
                        if attr_value.len() > 1
                            && !attr_value.chars().all(|c| c.is_whitespace())
                            && !seen_texts.contains(attr_value)
                        {
                            texts.push(attr_value.to_string());
                            seen_texts.insert(attr_value.to_string());
                        }
                    }

                    // 特殊处理iframe的Base64编码内容
                    if tag_name == "iframe"
                        && attr_name == "src"
                        && attr_value.contains("data:text/html;")
                    {
                        if let Some(base64_content) = extract_base64_from_data_uri(attr_value) {
                            if let Ok(decoded_html) =
                                general_purpose::STANDARD.decode(&base64_content)
                            {
                                if let Ok(decoded_str) = String::from_utf8(decoded_html) {
                                    println!(
                                        "🔍 解析Base64编码的HTML内容 ({} 字符)",
                                        decoded_str.len()
                                    );
                                    extract_texts_from_html_string(
                                        &decoded_str,
                                        &mut texts,
                                        &mut seen_texts,
                                    );
                                }
                            }
                        }
                    }
                }

                // 处理JavaScript代码中的文本
                if tag_name == "script" {
                    // 我们仍需要遍历script标签的子节点来获取内容
                    for child in node.children.borrow().iter() {
                        if let NodeData::Text { ref contents } = child.data {
                            let js_code = contents.borrow().to_string();
                            extract_texts_from_javascript(
                                &js_code,
                                &js_string_regex,
                                &json_string_regex,
                                &mut texts,
                                &mut seen_texts,
                            );
                        }
                    }
                }
            }
            _ => {}
        }

        // 继续遍历子节点 (除了已处理的script内容)
        let tag_name = match &node.data {
            NodeData::Element { ref name, .. } => name.local.as_ref(),
            _ => "",
        };

        if tag_name != "script" {
            // script标签的内容已经单独处理
            for child in node.children.borrow().iter() {
                queue.push_back(child.clone());
            }
        }
    }

    texts
}

/// 从data URI中提取Base64内容
fn extract_base64_from_data_uri(data_uri: &str) -> Option<String> {
    if let Some(comma_pos) = data_uri.find(',') {
        Some(data_uri[comma_pos + 1..].to_string())
    } else {
        None
    }
}

/// 从HTML字符串中提取可翻译文本
fn extract_texts_from_html_string(
    html: &str,
    texts: &mut Vec<String>,
    seen_texts: &mut HashSet<String>,
) {
    // 简单的HTML文本提取正则表达式
    let html_text_regex = match Regex::new(r">([^<>{3,})<") {
        Ok(regex) => regex,
        Err(_) => {
            eprintln!("警告: 无法编译HTML文本正则表达式");
            return;
        }
    };

    for captures in html_text_regex.captures_iter(html) {
        if let Some(text_match) = captures.get(1) {
            let text = text_match.as_str().trim();
            if text.len() > 2
                && !text
                    .chars()
                    .all(|c| c.is_whitespace() || c.is_ascii_punctuation())
                && !seen_texts.contains(text)
            {
                println!("🎯 从Base64 HTML中提取: '{}'", text);
                texts.push(text.to_string());
                seen_texts.insert(text.to_string());
            }
        }
    }

    // 也查找常见的英文文本模式
    let english_phrase_regex = match Regex::new(r"[A-Z][a-z]+(?:\s+[A-Z]?[a-z]+)*") {
        Ok(regex) => regex,
        Err(_) => {
            eprintln!("警告: 无法编译英文短语正则表达式");
            return;
        }
    };

    for captures in english_phrase_regex.captures_iter(html) {
        if let Some(phrase_match) = captures.get(0) {
            let phrase = phrase_match.as_str().trim();
            if phrase.len() > 3 &&
               phrase.split_whitespace().count() <= 6 &&  // 避免提取过长的文本
               !seen_texts.contains(phrase)
            {
                println!("📝 从Base64 HTML中提取英文短语: '{}'", phrase);
                texts.push(phrase.to_string());
                seen_texts.insert(phrase.to_string());
            }
        }
    }
}

/// 从JavaScript代码中提取可翻译文本
fn extract_texts_from_javascript(
    js_code: &str,
    js_string_regex: &Regex,
    json_string_regex: &Regex,
    texts: &mut Vec<String>,
    seen_texts: &mut HashSet<String>,
) {
    // 提取JavaScript字符串字面量
    for captures in js_string_regex.captures_iter(js_code) {
        if let Some(string_match) = captures.get(1) {
            let text = string_match.as_str().trim();
            if is_translatable_text(text) && !seen_texts.contains(text) {
                println!("🔧 从JavaScript中提取: '{}'", text);
                texts.push(text.to_string());
                seen_texts.insert(text.to_string());
            }
        }
    }

    // 专门处理JSON对象中的文本值 (key: "text value" 模式)
    let json_value_regex = match Regex::new(r#""text":\s*"([^"]{3,})""#) {
        Ok(regex) => regex,
        Err(_) => {
            eprintln!("警告: 无法编译JSON值正则表达式");
            return;
        }
    };

    for captures in json_value_regex.captures_iter(js_code) {
        if let Some(value_match) = captures.get(1) {
            let text_value = value_match.as_str().trim();
            if is_translatable_text(text_value) && !seen_texts.contains(text_value) {
                println!("🔨 从JavaScript JSON \"text\"中提取: '{}'", text_value);
                texts.push(text_value.to_string());
                seen_texts.insert(text_value.to_string());
            }
        }
    }

    // 提取JSON属性名（可能包含可翻译文本）
    for captures in json_string_regex.captures_iter(js_code) {
        if let Some(prop_match) = captures.get(1) {
            let prop_name = prop_match.as_str().trim();
            if is_translatable_text(prop_name) && !seen_texts.contains(prop_name) {
                println!("🔨 从JavaScript JSON属性中提取: '{}'", prop_name);
                texts.push(prop_name.to_string());
                seen_texts.insert(prop_name.to_string());
            }
        }
    }

    // 额外的通用JSON字符串值提取 (处理各种键名)
    let generic_json_value_regex = match Regex::new(r#""([A-Za-z][^"]*?)":\s*"([^"]{3,})""#) {
        Ok(regex) => regex,
        Err(_) => {
            eprintln!("警告: 无法编译通用JSON值正则表达式");
            return;
        }
    };

    for captures in generic_json_value_regex.captures_iter(js_code) {
        if let Some(key_match) = captures.get(1) {
            if let Some(value_match) = captures.get(2) {
                let key = key_match.as_str();
                let value = value_match.as_str().trim();

                // 只提取可能是用户界面文本的键值对
                if (key == "text" || key == "title" || key == "name" || key == "description")
                    && is_translatable_text(value)
                    && !seen_texts.contains(value)
                {
                    println!("🎯 从JavaScript JSON \"{}\"中提取: '{}'", key, value);
                    texts.push(value.to_string());
                    seen_texts.insert(value.to_string());
                }
            }
        }
    }
}

/// 判断文本是否适合翻译
fn is_translatable_text(text: &str) -> bool {
    text.len() > 2 &&
    text.len() < 200 &&  // 避免过长的文本
    !text.chars().all(|c| c.is_whitespace() || c.is_ascii_punctuation() || c.is_ascii_digit()) &&
    !text.starts_with("http") &&  // 排除URL
    !text.starts_with("www.") &&  // 排除域名
    !text.contains("function") &&  // 排除函数定义
    !text.contains("var ") &&  // 排除变量定义
    text.split_whitespace().count() <= 10 // 避免过长的句子
}

/// 将翻译结果应用到DOM
fn apply_translations_to_dom(
    dom: RcDom,
    original_texts: &[String],
    translations: &[String],
) -> Result<RcDom> {
    use markup5ever_rcdom::NodeData;
    use std::collections::{HashMap, VecDeque};

    // 创建翻译映射表，添加调试信息
    let translation_map: HashMap<String, String> = original_texts
        .iter()
        .zip(translations.iter())
        .filter(|(_, trans)| !trans.is_empty())
        .map(|(orig, trans)| {
            println!("映射: '{}' -> '{}'", orig, trans);
            (orig.clone(), trans.clone())
        })
        .collect();

    println!("📝 创建翻译映射: {} 个翻译对", translation_map.len());

    // 遍历DOM并应用翻译
    let mut queue = VecDeque::new();
    let mut applied_count = 0;
    queue.push_back(dom.document.clone());

    while let Some(node) = queue.pop_front() {
        match node.data {
            NodeData::Text { ref contents } => {
                let text = contents.borrow().to_string();
                let trimmed = text.trim();
                if let Some(translation) = translation_map.get(trimmed) {
                    let mut content_ref = contents.borrow_mut();
                    content_ref.clear();
                    content_ref.push_slice(translation);
                    applied_count += 1;
                    println!("✅ 应用翻译: '{}' -> '{}'", trimmed, translation);
                } else if trimmed.len() > 1
                    && !trimmed
                        .chars()
                        .all(|c| c.is_whitespace() || c.is_ascii_punctuation())
                {
                    println!("❌ 未找到翻译: '{}'", trimmed);
                }
            }
            NodeData::Element {
                ref name,
                ref attrs,
                ..
            } => {
                let tag_name = name.local.as_ref();
                if !matches!(tag_name, "script" | "style" | "noscript") {
                    // 翻译属性
                    for attr in attrs.borrow_mut().iter_mut() {
                        let attr_name = attr.name.local.as_ref();
                        if matches!(attr_name, "title" | "alt" | "placeholder") {
                            let value = attr.value.trim().to_string(); // 避免借用问题
                            if let Some(translation) = translation_map.get(&value) {
                                attr.value = translation.clone().into();
                                applied_count += 1;
                                println!(
                                    "✅ 应用属性翻译: {}='{}' -> '{}'",
                                    attr_name, value, translation
                                );
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        // 继续遍历子节点
        for child in node.children.borrow().iter() {
            queue.push_back(child.clone());
        }
    }

    println!("🎯 总共应用了 {} 个翻译", applied_count);
    Ok(dom)
}

/// 序列化DOM为HTML字符串
fn serialize_dom_to_html(dom: RcDom) -> Result<String> {
    use html5ever::serialize::{serialize, SerializeOpts};
    use markup5ever_rcdom::SerializableHandle;
    use std::io::Cursor;

    let mut buffer = Vec::new();
    let cursor = Cursor::new(&mut buffer);

    serialize(
        cursor,
        &SerializableHandle::from(dom.document.clone()),
        SerializeOpts::default(),
    )
    .map_err(|e| anyhow::anyhow!("HTML序列化失败: {:?}", e))?;

    String::from_utf8(buffer).map_err(|e| anyhow::anyhow!("UTF-8转换失败: {}", e))
}
fn calculate_content_hash(content: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}

/// 高性能索引标记翻译
async fn indexed_batch_translation(
    texts: Vec<String>,
    api_url: &str,
    concurrent_batches: usize,
    verbose: bool,
) -> Result<Vec<String>> {
    use futures::future::join_all;
    use reqwest::Client;

    if texts.is_empty() {
        return Ok(vec![]);
    }

    // 创建HTTP客户端
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("创建HTTP客户端失败")?;

    // 将文本分成批次，每个批次包含多个文本项
    let batch_size = std::cmp::max(5, texts.len() / concurrent_batches.max(1));
    let batches: Vec<_> = texts
        .chunks(batch_size)
        .enumerate()
        .map(|(batch_idx, chunk)| {
            // 为每个批次创建索引标记的文本
            let indexed_text = chunk
                .iter()
                .enumerate()
                .map(|(i, text)| format!("[{}] {}", batch_idx * batch_size + i, text.trim()))
                .collect::<Vec<_>>()
                .join("\n");
            (batch_idx, indexed_text, chunk.len())
        })
        .collect();

    if verbose {
        info!(
            "🚀 索引翻译: {} 个文本项分成 {} 个批次",
            texts.len(),
            batches.len()
        );
    }

    // 并发处理所有批次
    let tasks = batches.into_iter().map(|(batch_idx, indexed_text, count)| {
        let client = client.clone();
        let api_url = api_url.to_string();
        let verbose = verbose;

        async move {
            if verbose {
                info!("处理批次 {}: {} 个文本项", batch_idx + 1, count);
            }

            let result = translate_indexed_batch(&client, &api_url, &indexed_text).await;

            match &result {
                Ok(translations) => {
                    if verbose {
                        info!(
                            "✅ 批次 {} 完成: {} 个翻译",
                            batch_idx + 1,
                            translations.len()
                        );
                    }
                }
                Err(e) => {
                    warn!("❌ 批次 {} 失败: {}", batch_idx + 1, e);
                }
            }

            result
        }
    });

    // 等待所有批次完成
    let results = join_all(tasks).await;

    // 收集翻译结果
    let mut final_translations = vec![String::new(); texts.len()];
    let mut success_count = 0;

    for result in results {
        match result {
            Ok(batch_translations) => {
                for (global_index, translation) in batch_translations {
                    if global_index < final_translations.len() {
                        final_translations[global_index] = translation;
                        success_count += 1;
                    }
                }
            }
            Err(e) => {
                warn!("批次翻译失败: {}", e);
            }
        }
    }

    if verbose {
        let success_rate = success_count as f32 / texts.len() as f32 * 100.0;
        info!(
            "📊 索引翻译完成: 成功率 {:.1}% ({}/{})",
            success_rate,
            success_count,
            texts.len()
        );
    }

    Ok(final_translations)
}

/// 翻译单个索引批次
async fn translate_indexed_batch(
    client: &reqwest::Client,
    api_url: &str,
    indexed_text: &str,
) -> Result<Vec<(usize, String)>> {
    use regex::Regex;
    use serde_json::json;

    // 发送翻译请求
    let response = client
        .post(api_url)
        .json(&json!({
            "text": indexed_text,
            "source_lang": "auto",
            "target_lang": "zh"
        }))
        .send()
        .await
        .context("发送翻译请求失败")?;

    if !response.status().is_success() {
        anyhow::bail!("翻译API返回错误状态: {}", response.status());
    }

    let response_text = response.text().await.context("读取响应失败")?;

    // 尝试解析JSON响应
    let translated_text =
        if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&response_text) {
            json_val
                .get("data")
                .or_else(|| json_val.get("text"))
                .or_else(|| json_val.get("result"))
                .and_then(|v| v.as_str())
                .unwrap_or(&response_text)
                .to_string()
        } else {
            response_text
        };

    // 解析索引标记的翻译结果
    let index_regex = Regex::new(r"^\[(\d+)\]\s*(.*)$").context("编译正则表达式失败")?;
    let mut translations = Vec::new();

    for line in translated_text.lines() {
        if let Some(captures) = index_regex.captures(line.trim()) {
            if let (Some(index_str), Some(text)) = (captures.get(1), captures.get(2)) {
                if let Ok(index) = index_str.as_str().parse::<usize>() {
                    let translated = text.as_str().trim();
                    if !translated.is_empty() {
                        translations.push((index, translated.to_string()));
                    }
                }
            }
        }
    }

    Ok(translations)
}
