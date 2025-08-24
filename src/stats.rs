use std::time::Duration;

/// 自定义统计结构
#[derive(Debug)]
pub struct TranslationStats {
    pub config_time: Duration,
    pub translator_init_time: Duration,
    pub file_read_time: Duration,
    pub translation_time: Duration,
    pub file_write_time: Duration,
    pub input_size: usize,
    pub output_size: usize,
    pub texts_collected: usize,
    pub texts_filtered: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub batches_created: usize,
}

/// 打印性能统计
pub fn print_performance_stats(stats: &TranslationStats, total_duration: Duration) {
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
        801..=1000 => "✅达标",
        _ => "⚠️  需优化",
    };
    println!("   性能评级: {}", performance_grade);
}

/// 格式化持续时间
pub fn format_duration(duration: Duration) -> String {
    let millis = duration.as_millis();
    if millis < 1000 {
        format!("{}ms", millis)
    } else {
        format!("{:.3}s", duration.as_secs_f64())
    }
}