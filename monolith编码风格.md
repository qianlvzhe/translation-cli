# Monolith Rust项目编码风格模板

本文档基于对Monolith项目的深入分析，提供了一个全面的Rust编码风格指南，可用于指导LLM生成符合该项目风格的代码。

## 项目结构与组织

### 目录结构
```
项目根目录/
├── Cargo.toml          # 项目配置文件
├── src/                # 核心源代码
│   ├── lib.rs         # 库入口文件，导出模块
│   ├── main.rs        # 可执行文件入口
│   ├── core.rs        # 核心功能模块
│   ├── html.rs        # HTML处理模块
│   ├── css.rs         # CSS处理模块
│   ├── js.rs          # JavaScript处理模块
│   ├── url.rs         # URL处理工具
│   ├── session.rs     # 网络会话管理
│   ├── cache.rs       # 缓存系统
│   ├── cookies.rs     # Cookie处理
│   └── gui.rs         # GUI界面（可选功能）
├── tests/             # 测试代码
│   ├── mod.rs         # 测试模块入口
│   ├── cli/           # CLI功能测试
│   ├── core/          # 核心功能测试
│   ├── html/          # HTML处理测试
│   └── ...            # 其他功能测试
└── assets/            # 静态资源文件
```

### 模块组织原则
- 每个模块负责单一职责
- 使用`pub mod module_name`在lib.rs中导出模块
- 测试代码与源代码结构对应

## Cargo.toml配置风格

### 项目元信息
```toml
[package]
name = "project-name"
version = "x.y.z"
authors = ["Author Name <email@example.com>"]
edition = "2021"
description = "清晰简洁的项目描述"
homepage = "https://github.com/user/repo"
repository = "https://github.com/user/repo" 
readme = "README.md"
keywords = ["关键词", "列表"]
categories = ["category1", "category2"]
include = ["src/*.rs", "Cargo.toml"]
license = "许可证类型"
```

### 依赖管理
- **精确版本控制**: 使用`=`指定确切版本（如`"=0.2.14"`）
- **特性控制**: 明确指定需要的特性
```toml
[dependencies]
clap = { version = "=4.5.37", features = ["derive"], optional = true }
reqwest = { version = "=0.12.15", default-features = false, features = ["default-tls", "blocking"] }
```

### 条件编译特性
```toml
[features]
default = ["cli", "feature-name"]
cli = ["clap", "tempfile"]
gui = ["directories", "druid", "tempfile"]
```

### 多二进制文件配置
```toml
[[bin]]
name = "main-binary"
path = "src/main.rs"
required-features = ["cli"]

[[bin]]  
name = "gui-binary"
path = "src/gui.rs"
required-features = ["gui"]
```

## 代码命名约定

### 命名规则
- **类型名称**: PascalCase (`MonolithOptions`, `LinkType`)
- **函数名称**: snake_case (`create_data_url`, `parse_content_type`)
- **变量名称**: snake_case (`base_url`, `document_encoding`)
- **常量名称**: SCREAMING_SNAKE_CASE (`DEFAULT_USER_AGENT`, `CACHE_ASSET_FILE_SIZE_THRESHOLD`)
- **模块名称**: snake_case (`html`, `core`, `url`)

### 语义化命名
- 使用描述性名称: `detect_media_type` 而不是 `detect_type`
- 布尔值使用is/has前缀: `is_url_and_has_protocol`, `has_favicon`
- 动作使用动词: `create_`, `parse_`, `retrieve_`, `serialize_`

### 类型定义风格
```rust
// 枚举定义
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum OutputFormat {
    #[default]
    HTML,
    MHTML,
}

// 结构体定义
#[derive(Default)]
pub struct Options {
    pub base_url: Option<String>,
    pub timeout: u64,
    pub silent: bool,
}
```

## 错误处理模式

### 自定义错误类型
```rust
#[derive(Debug)]
pub struct CustomError {
    details: String,
}

impl CustomError {
    fn new(msg: &str) -> CustomError {
        CustomError {
            details: msg.to_string(),
        }
    }
}

impl fmt::Display for CustomError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl Error for CustomError {
    fn description(&self) -> &str {
        &self.details
    }
}
```

### Result类型使用
- 函数返回: `Result<(Vec<u8>, Url, String, String), reqwest::Error>`
- 错误传播: 使用`?`操作符
- 适当使用`unwrap()`和`expect()`，但添加有意义的错误信息

### 错误处理策略
- 对于可恢复错误使用Result
- 对于程序逻辑错误使用panic!
- 提供有意义的错误信息

## 模块可见性规则

### 公开接口设计
```rust
// 在lib.rs中导出模块
pub mod cache;
pub mod cookies;
pub mod core;

// 公开函数
pub fn create_monolithic_document(
    session: Session,
    target: String,
) -> Result<(Vec<u8>, Option<String>), MonolithError>

// 公开结构体
pub struct Session {
    cache: Option<Cache>,      // 私有字段
    pub options: Options,      // 公开字段
}
```

### 可见性原则
- 最小化公开接口
- 仅导出必要的类型和函数
- 使用`pub(crate)`为内部可见性

## 测试代码风格

### 测试模块组织
```rust
//  ASCII艺术注释头部
//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  (省略其他行...)

#[cfg(test)]
mod passing {
    use crate::module_name;

    #[test]
    fn descriptive_test_name() {
        // 测试逻辑
        assert_eq!(expected, actual);
    }
}

#[cfg(test)]
mod failing {
    // 失败测试用例
}
```

### 测试命名和结构
- 使用`passing`和`failing`模块分组
- 测试函数名称描述性强
- 使用相关的断言宏

### ASCII艺术头部
每个测试文件都包含标准的ASCII艺术"PASSING"头部注释

## 文档注释风格

### 文档注释使用
- 主要在main.rs中为CLI参数使用`///`注释
- 保持注释简洁明了
- 避免过度文档化内部实现

### 注释示例
```rust
/// Remove audio sources
#[arg(short = 'a', long)]
no_audio: bool,

/// Set custom base URL
#[arg(short, long, value_name = "http://localhost/")]
base_url: Option<String>,
```

## 代码格式化标准

### 导入语句组织
```rust
// 标准库导入
use std::fs;
use std::io::{self, Error as IoError, Read, Write};
use std::process;

// 第三方crate导入
use clap::Parser;
use tempfile::{Builder, NamedTempFile};

// 本地模块导入
use monolith::cache::Cache;
use monolith::core::{create_monolithic_document, MonolithOptions};
```

### 函数定义风格
```rust
pub fn function_name(
    param1: Type1,
    param2: Type2,
) -> ReturnType {
    // 函数体
}
```

### 常量定义
```rust
const CONSTANT_NAME: &str = "value";
const NUMERIC_CONSTANT: usize = 1024 * 10;
const ARRAY_CONSTANT: [[&[u8]; 2]; 18] = [
    [b"signature", b"media/type"],
    // ...
];
```

## 特定模式和约定

### URL处理模式
```rust
pub fn clean_url(url: Url) -> Url {
    let mut result = url.clone();
    result.set_fragment(None);
    result
}
```

### Session模式
```rust
impl Session {
    pub fn new(cache: Option<Cache>, options: Options) -> Self {
        // 初始化逻辑
    }
    
    pub fn retrieve_asset(&mut self, url: &Url) -> Result<DataType, ErrorType> {
        // 实现逻辑
    }
}
```

### 数据处理管道
- 输入验证 → 处理 → 输出格式化
- 使用中间结果缓存
- 错误在各阶段适当处理

## 性能考虑

### 内存管理
- 使用引用避免不必要的克隆
- 适当使用`Vec::with_capacity()`预分配内存
- 使用`Cow`类型处理可能的所有权转移

### 并发和异步
- 本项目主要使用同步阻塞模式
- reqwest使用`blocking`特性

## 编码最佳实践

### 代码组织
1. 保持函数简洁，单一职责
2. 使用有意义的中间变量
3. 适当的代码分层

### 安全性
- 输入验证和清理
- 避免unsafe代码
- 使用类型系统确保安全性

### 可维护性
- 清晰的模块边界
- 最小化模块间依赖
- 一致的错误处理策略

---

## 使用指南

当使用此模板生成代码时，请确保：

1. **遵循命名约定**: 使用snake_case和PascalCase
2. **保持模块结构**: 功能相关代码组织在同一模块
3. **错误处理**: 使用Result类型和适当的错误传播
4. **测试覆盖**: 为新功能添加对应测试
5. **文档注释**: 为公开API提供清晰文档
6. **依赖管理**: 使用精确版本控制
7. **性能考虑**: 避免不必要的内存分配和克隆

这个模板捕获了Monolith项目的核心编码风格和架构模式，可以作为生成一致性代码的指南。