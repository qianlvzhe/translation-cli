use anyhow::Result;
use std::path::PathBuf;
use tracing::warn;

/// 初始化日志系统
pub fn init_logging(verbose: bool, quiet: bool) {
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
pub fn validate_input_file(path: &PathBuf) -> Result<()> {
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
pub fn generate_output_path(input: &PathBuf, output: &Option<PathBuf>, lang: &str) -> PathBuf {
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

/// 判断文本是否适合翻译
pub fn is_translatable_text(text: &str) -> bool {
    text.len() > 2 &&
    text.len() < 200 &&  // 避免过长的文本
    !text.chars().all(|c| c.is_whitespace() || c.is_ascii_punctuation() || c.is_ascii_digit()) &&
    !text.starts_with("http") &&  // 排除URL
    !text.starts_with("www.") &&  // 排除域名
    !text.contains("function") &&  // 排除函数定义
    !text.contains("var ") &&  // 排除变量定义
    text.split_whitespace().count() <= 10 // 避免过长的句子
}

/// 从data URI中提取Base64内容
pub fn extract_base64_from_data_uri(data_uri: &str) -> Option<String> {
    if let Some(comma_pos) = data_uri.find(',') {
        Some(data_uri[comma_pos + 1..].to_string())
    } else {
        None
    }
}

/// 计算内容哈希值
pub fn calculate_content_hash(content: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}