//! 通用工具函数模块
//!
//! 提供输入验证、路径生成、文本处理等通用功能

// 标准库导入
use std::path::PathBuf;

// 第三方crate导入
use anyhow::Result;
use tracing::warn;
use url::Url;

/// 输入源类型枚举
/// 
/// 表示翻译工具支持的两种输入类型，用于统一处理
/// 本地文件和网页URL的翻译需求。
/// 
/// # Variants
/// 
/// * `File` - 本地HTML文件路径
/// * `Url` - 网页URL地址，支持HTTP和HTTPS协议
#[derive(Debug, Clone)]
pub enum InputSource {
    /// 本地文件路径
    File(PathBuf),
    /// 网页URL
    Url(Url),
}

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

/// 验证并解析输入源
/// 
/// 自动识别输入字符串是本地文件路径还是URL地址，
/// 并返回相应的输入源类型。
/// 
/// # 识别逻辑
/// 
/// 1. 首先尝试解析为URL
/// 2. 检查是否为支持的HTTP/HTTPS协议
/// 3. 如果不是URL，则处理为文件路径
/// 4. 自动将相对路径转换为绝对路径
/// 
/// # Arguments
/// 
/// * `input` - 输入字符串（文件路径或URL地址）
/// 
/// # Returns
/// 
/// * `Result<InputSource>` - 成功时返回解析后的输入源类型
/// 
/// # Errors
/// 
/// * 当无法获取当前工作目录时返回错误
/// 
/// # Examples
/// 
/// ```rust
/// use translation_cli::utils::validate_input_source;
/// 
/// // URL输入
/// let url_source = validate_input_source("https://example.com").unwrap();
/// match url_source {
///     InputSource::Url(url) => println!("检测到URL: {}", url),
///     _ => unreachable!(),
/// }
/// 
/// // 文件路径输入  
/// let file_source = validate_input_source("test.html").unwrap();
/// match file_source {
///     InputSource::File(path) => println!("检测到文件: {}", path.display()),
///     _ => unreachable!(),
/// }
/// ```
pub fn validate_input_source(input: &str) -> Result<InputSource> {
    // 先尝试解析为URL
    if let Ok(url) = Url::parse(input) {
        // 检查是否为HTTP/HTTPS URL
        if url.scheme() == "http" || url.scheme() == "https" {
            return Ok(InputSource::Url(url));
        }
    }
    
    // 尝试作为文件路径处理
    let path = PathBuf::from(input);
    
    // 如果是相对路径，转换为绝对路径
    let absolute_path = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()?.join(path)
    };
    
    Ok(InputSource::File(absolute_path))
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

/// 为不同输入源生成输出路径
pub fn generate_output_path_for_source(source: &InputSource, output: &Option<PathBuf>, lang: &str) -> PathBuf {
    if let Some(output_path) = output {
        return output_path.clone();
    }

    match source {
        InputSource::File(path) => {
            // 对于文件，使用现有逻辑
            generate_output_path(path, &None, lang)
        },
        InputSource::Url(url) => {
            // 对于URL，使用域名和路径生成文件名
            let host = url.host_str().unwrap_or("webpage");
            let path_segments: Vec<&str> = url.path_segments()
                .map(|segments| segments.filter(|s| !s.is_empty()).collect())
                .unwrap_or_default();
            
            let filename = if path_segments.is_empty() {
                format!("{}_{}_{}.html", host, "index", lang)
            } else {
                let page_name = path_segments.last().unwrap_or(&"page");
                // 移除文件扩展名（如果有的话）
                let page_name = if let Some(dot_pos) = page_name.rfind('.') {
                    &page_name[..dot_pos]
                } else {
                    page_name
                };
                format!("{}_{}_{}.html", host, page_name, lang)
            };
            
            // 清理文件名中的非法字符
            let safe_filename = filename
                .chars()
                .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' || c == '.' { c } else { '_' })
                .collect::<String>();
            
            PathBuf::from(safe_filename)
        }
    }
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