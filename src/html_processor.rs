//! HTML处理模块
//!
//! 提供HTML解析、文本提取、DOM操作和序列化功能

// 标准库导入
use std::collections::{HashMap, HashSet, VecDeque};

// 第三方crate导入
use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use markup5ever_rcdom::{NodeData, RcDom};
use regex::Regex;

// 本地模块导入
use crate::utils::{is_translatable_text, extract_base64_from_data_uri};

/// 提取DOM中的可翻译文本
pub fn extract_translatable_texts(dom: &RcDom) -> Vec<String> {
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

/// 将翻译结果应用到DOM
pub fn apply_translations_to_dom(
    dom: RcDom,
    original_texts: &[String],
    translations: &[String],
) -> Result<RcDom> {
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
pub fn serialize_dom_to_html(dom: RcDom) -> Result<String> {
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