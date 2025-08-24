use anyhow::{Context, Result};
use futures::future::join_all;
use html5ever::parse_document;
use html5ever::tendril::TendrilSink;
use markup5ever_rcdom::RcDom;
use regex::Regex;
use reqwest::Client;
use serde_json::json;
use tracing::{info, warn};

use crate::html_processor::{extract_translatable_texts, apply_translations_to_dom, serialize_dom_to_html};

/// 使用索引模式的高性能翻译
pub async fn translate_with_indexed_mode(
    html_content: &str,
    api_url: &str,
    concurrent_batches: usize,
    verbose: bool,
) -> Result<String> {
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

/// 高性能索引标记翻译
pub async fn indexed_batch_translation(
    texts: Vec<String>,
    api_url: &str,
    concurrent_batches: usize,
    verbose: bool,
) -> Result<Vec<String>> {
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
pub async fn translate_indexed_batch(
    client: &reqwest::Client,
    api_url: &str,
    indexed_text: &str,
) -> Result<Vec<(usize, String)>> {
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