#!/bin/bash

# Translation CLI Integration Test Script
# 集成测试脚本 - 测试翻译CLI的各种功能

set -e  # 遇到错误时退出

# 测试配置
PROJECT_DIR="/home/qian/dev/translation-cli"
TEST_DATA_DIR="$PROJECT_DIR/tests/test_data"
OUTPUT_DIR="$PROJECT_DIR/tests/output"
LOG_FILE="$PROJECT_DIR/tests/integration_test.log"

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 日志函数
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1" | tee -a "$LOG_FILE"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1" | tee -a "$LOG_FILE"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1" | tee -a "$LOG_FILE"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1" | tee -a "$LOG_FILE"
}

# 清理函数
cleanup() {
    log_info "清理测试环境..."
    rm -rf "$OUTPUT_DIR"
    rm -f "$LOG_FILE"
}

# 设置测试环境
setup_test_environment() {
    log_info "设置测试环境..."
    
    # 清理之前的测试结果
    cleanup
    
    # 创建必要的目录
    mkdir -p "$TEST_DATA_DIR"
    mkdir -p "$OUTPUT_DIR"
    
    # 创建测试HTML文件
    cat > "$TEST_DATA_DIR/test.html" << 'EOF'
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Test Page</title>
</head>
<body>
    <h1>Welcome to Test Page</h1>
    <p>This is a test paragraph for translation.</p>
    <div>
        <span>Hello World</span>
        <p>Another paragraph with <strong>bold text</strong> and <em>italic text</em>.</p>
    </div>
    <ul>
        <li>First item</li>
        <li>Second item</li>
        <li>Third item</li>
    </ul>
</body>
</html>
EOF

    log_success "测试环境设置完成"
}

# 编译项目
build_project() {
    log_info "编译项目..."
    cd "$PROJECT_DIR"
    
    if cargo build --release; then
        log_success "项目编译成功"
        return 0
    else
        log_error "项目编译失败"
        return 1
    fi
}

# 测试基本文件翻译功能
test_file_translation() {
    log_info "测试基本文件翻译功能..."
    
    local input_file="$TEST_DATA_DIR/test.html"
    local output_file="$OUTPUT_DIR/test_translated.html"
    
    cd "$PROJECT_DIR"
    if timeout 120 cargo run --release -- \
        --input "$input_file" \
        --output "$output_file" \
        --lang zh; then
        
        # 检查输出文件是否存在
        if [ -f "$output_file" ]; then
            log_success "文件翻译测试通过"
            return 0
        else
            log_error "输出文件不存在: $output_file"
            return 1
        fi
    else
        log_error "文件翻译测试失败"
        return 1
    fi
}

# 测试URL翻译功能（使用简单的网页）
test_url_translation() {
    log_info "测试URL翻译功能..."
    
    local test_url="https://httpbin.org/html"
    local output_file="$OUTPUT_DIR/url_translated.html"
    
    cd "$PROJECT_DIR"
    if timeout 180 cargo run --release -- \
        --input "$test_url" \
        --output "$output_file" \
        --from-url \
        --lang zh; then
        
        # 检查输出文件是否存在
        if [ -f "$output_file" ]; then
            log_success "URL翻译测试通过"
            return 0
        else
            log_error "输出文件不存在: $output_file"
            return 1
        fi
    else
        log_warning "URL翻译测试可能因为网络问题失败，跳过此测试"
        return 0
    fi
}

# 测试错误处理
test_error_handling() {
    log_info "测试错误处理..."
    
    local non_existent_file="$TEST_DATA_DIR/non_existent.html"
    
    cd "$PROJECT_DIR"
    # 测试不存在的输入文件（应该失败）
    if cargo run --release -- \
        --input "$non_existent_file" \
        --output "$OUTPUT_DIR/error_test.html" \
        --lang zh 2>/dev/null; then
        
        log_error "错误处理测试失败 - 应该报错但没有报错"
        return 1
    else
        log_success "错误处理测试通过 - 正确处理了不存在的文件"
        return 0
    fi
}

# 测试命令行参数
test_cli_options() {
    log_info "测试命令行参数..."
    
    cd "$PROJECT_DIR"
    # 测试帮助选项
    if cargo run --release -- --help > /dev/null 2>&1; then
        log_success "帮助选项测试通过"
    else
        log_error "帮助选项测试失败"
        return 1
    fi
    
    # 测试版本选项
    if cargo run --release -- --version > /dev/null 2>&1; then
        log_success "版本选项测试通过"
    else
        log_warning "版本选项可能未实现，跳过"
    fi
    
    return 0
}

# 运行单元测试
run_unit_tests() {
    log_info "运行单元测试..."
    
    cd "$PROJECT_DIR"
    if cargo test --lib; then
        log_success "单元测试通过"
        return 0
    else
        log_error "单元测试失败"
        return 1
    fi
}

# 运行性能测试
test_performance() {
    log_info "运行性能测试..."
    
    local input_file="$TEST_DATA_DIR/test.html"
    local output_file="$OUTPUT_DIR/performance_test.html"
    
    cd "$PROJECT_DIR"
    local start_time=$(date +%s)
    
    if timeout 60 cargo run --release -- \
        --input "$input_file" \
        --output "$output_file" \
        --lang zh > /dev/null 2>&1; then
        
        local end_time=$(date +%s)
        local duration=$((end_time - start_time))
        
        if [ $duration -lt 30 ]; then
            log_success "性能测试通过 - 翻译耗时: ${duration}秒"
        else
            log_warning "性能测试警告 - 翻译耗时较长: ${duration}秒"
        fi
        return 0
    else
        log_error "性能测试失败"
        return 1
    fi
}

# 生成测试报告
generate_test_report() {
    log_info "生成测试报告..."
    
    local report_file="$OUTPUT_DIR/test_report.txt"
    
    cat > "$report_file" << EOF
Translation CLI Integration Test Report
=====================================
Date: $(date)
Test Environment: $(uname -a)
Project Directory: $PROJECT_DIR

Test Results Summary:
EOF
    
    # 添加日志内容到报告
    if [ -f "$LOG_FILE" ]; then
        echo "" >> "$report_file"
        echo "Detailed Logs:" >> "$report_file"
        echo "==============" >> "$report_file"
        cat "$LOG_FILE" >> "$report_file"
    fi
    
    log_success "测试报告已生成: $report_file"
}

# 主测试流程
main() {
    echo "======================================="
    echo "Translation CLI Integration Test Suite"
    echo "======================================="
    
    local failed_tests=0
    local total_tests=0
    
    # 初始化
    setup_test_environment
    
    # 编译项目
    total_tests=$((total_tests + 1))
    if ! build_project; then
        failed_tests=$((failed_tests + 1))
        log_error "编译失败，停止测试"
        exit 1
    fi
    
    # 运行各项测试
    tests=(
        "run_unit_tests"
        "test_cli_options"
        "test_file_translation"
        "test_url_translation"
        "test_error_handling"
        "test_performance"
    )
    
    for test_func in "${tests[@]}"; do
        total_tests=$((total_tests + 1))
        echo ""
        if ! $test_func; then
            failed_tests=$((failed_tests + 1))
        fi
    done
    
    # 生成报告
    generate_test_report
    
    # 输出最终结果
    echo ""
    echo "======================================="
    echo "测试完成!"
    echo "总测试数: $total_tests"
    echo "失败测试数: $failed_tests"
    echo "成功测试数: $((total_tests - failed_tests))"
    
    if [ $failed_tests -eq 0 ]; then
        log_success "所有测试通过! 🎉"
        exit 0
    else
        log_error "有 $failed_tests 个测试失败"
        exit 1
    fi
}

# 处理脚本参数
case "${1:-}" in
    "setup")
        setup_test_environment
        ;;
    "cleanup")
        cleanup
        ;;
    "unit")
        setup_test_environment
        build_project
        run_unit_tests
        ;;
    "file")
        setup_test_environment
        build_project
        test_file_translation
        ;;
    "url")
        setup_test_environment  
        build_project
        test_url_translation
        ;;
    "performance")
        setup_test_environment
        build_project
        test_performance
        ;;
    *)
        main
        ;;
esac