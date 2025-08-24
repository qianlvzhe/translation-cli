# Translation CLI

高性能HTML翻译命令行工具，基于 `html-translation-lib` 库构建，支持亚秒级文件翻译。

## 项目概述

Translation CLI 是一个专门用于HTML文件翻译的命令行工具，专注于提供高性能的翻译体验。该工具支持多种翻译模式，具备智能文本提取、批处理优化、并发处理等特性。

## 核心特性

### 🚀 高性能翻译
- **亚秒级目标**: 针对小到中等大小的HTML文件，努力实现1秒内完成翻译
- **智能批处理**: 动态调整批次大小，优化API调用效率
- **并发处理**: 支持多批次并发翻译，大幅提升处理速度
- **性能统计**: 详细的性能分析报告，包含时间分解和效率指标

### 🎯 智能文本处理
- **DOM解析**: 使用 `html5ever` 进行精确的HTML解析
- **文本过滤**: 智能识别和过滤可翻译内容，避免翻译代码和标记
- **属性翻译**: 支持 `title`、`alt`、`placeholder` 等HTML属性翻译
- **JavaScript提取**: 能够提取JavaScript代码中的可翻译字符串
- **Base64内容处理**: 支持解析和翻译Base64编码的HTML内容

### 🔧 灵活配置
- **多API支持**: 支持自定义翻译API或使用本地API服务
- **语言选择**: 支持多种目标语言（默认中文）
- **缓存机制**: 可选的翻译缓存，减少重复翻译
- **文件命名**: 自动生成带语言标识的输出文件名

### 📊 监控与诊断
- **详细日志**: 分级日志输出（静默/普通/详细）
- **性能监控**: 实时性能指标和优化建议
- **错误处理**: 完善的错误处理和重试机制
- **统计报告**: 文件大小、翻译数量、缓存命中率等统计信息

## 安装

### 从源码构建

```bash
git clone <repository-url>
cd translation-cli
cargo build --release
```

构建完成后，可执行文件位于 `target/release/translation-cli`

### 依赖要求

- Rust 1.70+
- `html-translation-lib` 库（版本 0.1.0）

## 使用方法

### 基本用法

```bash
# 翻译HTML文件到中文（默认）
translation-cli -i input.html

# 指定输出文件和目标语言
translation-cli -i input.html -o output.html -l ja

# 使用本地API服务
translation-cli -i input.html --local-api

# 启用详细模式和性能统计
translation-cli -i input.html --verbose --stats
```

### 高性能模式

```bash
# 启用索引翻译模式（推荐用于大文件）
translation-cli -i large.html --indexed-translation --concurrent-batches 10

# 大批处理模式
translation-cli -i input.html --large-batch --batch-size 100
```

### 命令行选项

| 选项 | 简写 | 说明 | 默认值 |
|------|------|------|--------|
| `--input` | `-i` | 输入HTML文件路径 | 必需 |
| `--output` | `-o` | 输出文件路径 | 自动生成 |
| `--lang` | `-l` | 目标语言代码 | `zh` |
| `--api` | `-a` | 翻译API地址 | `****` |
| `--batch-size` |  | 批处理大小 | `25` |
| `--max-retries` |  | 最大重试次数 | `3` |
| `--no-cache` |  | 禁用缓存 | false |
| `--verbose` | `-v` | 详细输出 | false |
| `--quiet` | `-q` | 静默模式 | false |
| `--stats` |  | 显示性能统计 | false |
| `--large-batch` |  | 大批处理模式 | false |
| `--local-api` |  | 使用本地API | false |
| `--indexed-translation` |  | 索引翻译模式 | false |
| `--concurrent-batches` |  | 并发批次数量 | `5` |

## 工作原理

### 翻译流程

1. **文件验证**: 检查输入文件存在性和格式
2. **HTML解析**: 使用DOM解析器分析HTML结构
3. **文本提取**: 智能提取可翻译的文本内容
4. **批处理**: 将文本分组为批次以优化API调用
5. **并发翻译**: 同时处理多个批次
6. **结果应用**: 将翻译结果应用回DOM结构
7. **文件输出**: 序列化并保存翻译后的HTML

### 两种翻译模式

#### 1. 标准模式
使用 `html-translation-lib` 库进行翻译，适合一般用途：

```bash
translation-cli -i input.html
```

#### 2. 索引翻译模式
高性能模式，使用索引标记进行批量翻译：

```bash
translation-cli -i input.html --indexed-translation
```

### 文本提取逻辑

- **HTML文本节点**: 提取标签间的文本内容
- **HTML属性**: 提取 `title`、`alt`、`placeholder` 属性值
- **JavaScript字符串**: 提取JS代码中的字符串字面量
- **JSON对象**: 提取JSON中的文本值
- **Base64内容**: 解码并提取其中的HTML文本

### 性能优化

- **智能过滤**: 避免翻译代码、URL、变量名等
- **批处理优化**: 动态调整批次大小
- **并发控制**: 可配置的并发批次数量
- **缓存机制**: 避免重复翻译相同内容

## 性能指标

### 性能目标
- **亚秒级**: 小文件(<50KB) 目标 <1000ms
- **高效率**: 大文件处理速度 >100KB/s

### 性能评级
- 🏆 **优秀**: <500ms
- 👍 **良好**: 501-800ms
- ✅ **达标**: 801-1000ms
- ⚠️ **需优化**: >1000ms

### 统计信息
工具会显示详细的性能统计，包括：
- 时间分解（读取、翻译、写入等）
- 文件大小变化
- 文本处理统计
- 缓存命中率
- 处理速度

## 配置示例

### 本地开发配置
```bash
# 使用本地API服务，启用详细日志
translation-cli -i test.html --local-api --verbose --stats
```

### 生产环境配置
```bash
# 大文件高性能翻译
translation-cli -i large.html \
  --indexed-translation \
  --concurrent-batches 10 \
  --large-batch \
  --api "https://api.translate.service.com/v1/translate"
```

### 批量处理
```bash
# 处理多个文件
for file in *.html; do
  translation-cli -i "$file" --quiet --stats
done
```

## 故障排除

### 常见问题

1. **翻译API连接失败**
   - 检查API地址是否正确
   - 确认网络连接
   - 尝试使用 `--local-api` 选项

2. **性能不佳**
   - 使用 `--indexed-translation` 模式
   - 增加 `--concurrent-batches` 数量
   - 启用 `--large-batch` 模式

3. **翻译质量问题**
   - 检查源文件HTML结构
   - 使用 `--verbose` 查看提取的文本
   - 调整批处理大小

### 调试模式
```bash
# 启用最详细的调试输出
translation-cli -i input.html --verbose --stats
```

## 开发说明

### 项目结构
```
translation-cli/
├── src/
│   └── main.rs          # 主程序入口
├── Cargo.toml           # 项目配置
├── target/              # 编译输出
└── README.md           # 项目文档
```

### 核心模块
- **CLI解析**: 使用 `clap` 处理命令行参数
- **HTML处理**: `html5ever` + `markup5ever_rcdom` 进行DOM操作
- **异步翻译**: `tokio` + `reqwest` 实现并发HTTP请求
- **错误处理**: `anyhow` 提供统一错误处理
- **日志系统**: `tracing` 实现结构化日志

### 扩展开发
该工具设计为可扩展的架构，可以轻松添加：
- 新的翻译API支持
- 额外的文件格式支持
- 自定义文本过滤规则
- 更多的性能优化策略

## 许可证

该项目是开源项目的一部分，具体许可证条款请参考项目根目录的LICENSE文件。

## 贡献

欢迎提交Issue和Pull Request来改进这个工具。在提交代码前，请确保：
- 代码通过所有测试
- 遵循项目的代码风格
- 添加适当的文档和注释

## 更新历史

### v0.1.0
- 初始版本发布
- 基本HTML翻译功能
- 索引翻译模式
- 性能监控和统计
- 命令行界面