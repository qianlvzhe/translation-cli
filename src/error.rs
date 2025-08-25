//! 统一错误处理模块
//!
//! 提供Translation CLI项目的统一错误类型定义和处理机制

// 标准库导入
use std::fmt;

// 第三方crate导入
use anyhow::Error as AnyhowError;

/// Translation CLI 统一错误类型
/// 
/// 定义了项目中可能出现的所有错误类型，提供统一的错误处理接口
#[derive(Debug)]
pub enum TranslationError {
    /// 网络请求相关错误
    Network { 
        /// 错误消息
        message: String, 
        /// HTTP状态码（如果适用）
        status_code: Option<u16> 
    },
    
    /// HTML解析相关错误
    HtmlParse { 
        /// 具体错误信息
        details: String 
    },
    
    /// 文件操作相关错误
    FileOperation { 
        /// 文件路径
        path: String, 
        /// 操作类型（读取、写入、创建等）
        operation: String, 
        /// 底层错误信息
        source: String 
    },
    
    /// 翻译API相关错误
    TranslationApi { 
        /// API响应状态码
        status_code: u16, 
        /// 错误消息
        message: String, 
        /// API地址
        api_url: String 
    },
    
    /// 配置相关错误
    Configuration { 
        /// 配置项名称
        field: String, 
        /// 错误原因
        reason: String 
    },
    
    /// 输入验证错误
    InputValidation { 
        /// 输入值
        input: String, 
        /// 验证失败原因
        reason: String 
    },
    
    /// 临时文件管理错误
    TempFileManagement { 
        /// 操作类型
        operation: String, 
        /// 错误详情
        details: String 
    },
    
    /// 内部处理错误（包装anyhow::Error）
    Internal { 
        /// 包装的错误
        source: AnyhowError 
    },
}

impl fmt::Display for TranslationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TranslationError::Network { message, status_code } => {
                if let Some(code) = status_code {
                    write!(f, "网络请求失败 [{}]: {}", code, message)
                } else {
                    write!(f, "网络请求失败: {}", message)
                }
            },
            TranslationError::HtmlParse { details } => {
                write!(f, "HTML解析失败: {}", details)
            },
            TranslationError::FileOperation { path, operation, source } => {
                write!(f, "文件{}操作失败 [{}]: {}", operation, path, source)
            },
            TranslationError::TranslationApi { status_code, message, api_url } => {
                write!(f, "翻译API错误 [{}] {}: {}", status_code, api_url, message)
            },
            TranslationError::Configuration { field, reason } => {
                write!(f, "配置错误 [{}]: {}", field, reason)
            },
            TranslationError::InputValidation { input, reason } => {
                write!(f, "输入验证失败 [{}]: {}", input, reason)
            },
            TranslationError::TempFileManagement { operation, details } => {
                write!(f, "临时文件{}失败: {}", operation, details)
            },
            TranslationError::Internal { source } => {
                write!(f, "内部处理错误: {}", source)
            },
        }
    }
}

impl std::error::Error for TranslationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TranslationError::Internal { source } => Some(source.as_ref()),
            _ => None,
        }
    }
}

/// Translation CLI 结果类型别名
pub type Result<T> = std::result::Result<T, TranslationError>;

/// 便捷的错误创建宏
#[macro_export]
macro_rules! translation_error {
    (network, $msg:expr) => {
        TranslationError::Network { 
            message: $msg.to_string(), 
            status_code: None 
        }
    };
    (network, $msg:expr, $code:expr) => {
        TranslationError::Network { 
            message: $msg.to_string(), 
            status_code: Some($code) 
        }
    };
    (html_parse, $details:expr) => {
        TranslationError::HtmlParse { 
            details: $details.to_string() 
        }
    };
    (file_op, $path:expr, $op:expr, $source:expr) => {
        TranslationError::FileOperation { 
            path: $path.to_string(), 
            operation: $op.to_string(), 
            source: $source.to_string() 
        }
    };
    (translation_api, $code:expr, $msg:expr, $url:expr) => {
        TranslationError::TranslationApi { 
            status_code: $code, 
            message: $msg.to_string(), 
            api_url: $url.to_string() 
        }
    };
    (config, $field:expr, $reason:expr) => {
        TranslationError::Configuration { 
            field: $field.to_string(), 
            reason: $reason.to_string() 
        }
    };
    (input_validation, $input:expr, $reason:expr) => {
        TranslationError::InputValidation { 
            input: $input.to_string(), 
            reason: $reason.to_string() 
        }
    };
    (temp_file, $op:expr, $details:expr) => {
        TranslationError::TempFileManagement { 
            operation: $op.to_string(), 
            details: $details.to_string() 
        }
    };
}

/// 从anyhow::Error转换为TranslationError
impl From<AnyhowError> for TranslationError {
    fn from(error: AnyhowError) -> Self {
        TranslationError::Internal { source: error }
    }
}

/// 从reqwest::Error转换为TranslationError
impl From<reqwest::Error> for TranslationError {
    fn from(error: reqwest::Error) -> Self {
        let status_code = error.status().map(|s| s.as_u16());
        TranslationError::Network {
            message: error.to_string(),
            status_code,
        }
    }
}

/// 从std::io::Error转换为TranslationError
impl From<std::io::Error> for TranslationError {
    fn from(error: std::io::Error) -> Self {
        TranslationError::FileOperation {
            path: "unknown".to_string(),
            operation: "io".to_string(),
            source: error.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = TranslationError::Network {
            message: "Connection failed".to_string(),
            status_code: Some(500),
        };
        
        assert_eq!(
            format!("{}", err),
            "网络请求失败 [500]: Connection failed"
        );
    }

    #[test]
    fn test_error_macro() {
        let err = translation_error!(network, "Test error", 404);
        match err {
            TranslationError::Network { message, status_code } => {
                assert_eq!(message, "Test error");
                assert_eq!(status_code, Some(404));
            },
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_anyhow_conversion() {
        let anyhow_err = anyhow::anyhow!("Test anyhow error");
        let translation_err: TranslationError = anyhow_err.into();
        
        match translation_err {
            TranslationError::Internal { .. } => {
                // Test passes
            },
            _ => panic!("Wrong error type"),
        }
    }
}