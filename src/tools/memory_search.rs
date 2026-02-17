use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::claude::ToolDefinition;
use crate::memory_decay;
use super::{schema_object, Tool, ToolResult};

pub struct MemorySearchTool {
    memory_dir: PathBuf,
}

impl MemorySearchTool {
    pub fn new(memory_dir: PathBuf) -> Self {
        Self { memory_dir }
    }
}

/// A scored memory entry for ranking.
#[derive(Debug)]
struct ScoredEntry {
    filename: String,
    content: String,
    bm25_score: f64,
    decay_score: f64,
    combined_score: f64,
}

/// BM25 parameters
const BM25_K1: f64 = 1.2;
const BM25_B: f64 = 0.75;

/// Tokenize text into lowercase words.
fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|w| w.len() >= 2)
        .map(|w| w.to_string())
        .collect()
}

/// Calculate BM25 score for a document against a query.
/// `doc_tokens`: tokens in the document
/// `query_tokens`: tokens in the query
/// `avg_doc_len`: average document length across corpus
/// `n_docs`: total number of documents
/// `doc_freq`: number of documents containing each term
fn bm25_score(
    doc_tokens: &[String],
    query_tokens: &[String],
    avg_doc_len: f64,
    n_docs: usize,
    doc_freq: &HashMap<String, usize>,
) -> f64 {
    let doc_len = doc_tokens.len() as f64;

    // Count term frequencies in document
    let mut tf: HashMap<&str, usize> = HashMap::new();
    for token in doc_tokens {
        *tf.entry(token.as_str()).or_insert(0) += 1;
    }

    let mut score = 0.0;
    for query_term in query_tokens {
        let term_freq = *tf.get(query_term.as_str()).unwrap_or(&0) as f64;
        if term_freq == 0.0 {
            continue;
        }

        let df = *doc_freq.get(query_term.as_str()).unwrap_or(&0) as f64;
        let n = n_docs as f64;

        // IDF: log((N - df + 0.5) / (df + 0.5))
        let idf = ((n - df + 0.5) / (df + 0.5)).ln();
        if idf <= 0.0 {
            continue;
        }

        // TF component: tf / (tf + k1 * (1 - b + b * doc_len / avg_doc_len))
        let tf_component =
            term_freq / (term_freq + BM25_K1 * (1.0 - BM25_B + BM25_B * doc_len / avg_doc_len));

        score += idf * tf_component;
    }

    score
}

/// Parse memory file into entries with timestamps.
/// Each entry starts with "## YYYY-MM-DD HH:MM:SS UTC"
fn parse_entries(content: &str, filename: &str) -> Vec<(String, Option<f64>)> {
    let mut entries = Vec::new();
    let mut current_entry = String::new();
    let mut current_timestamp: Option<String> = None;

    for line in content.lines() {
        if line.starts_with("## ") {
            // Save previous entry
            if !current_entry.trim().is_empty() {
                let age = current_timestamp
                    .as_deref()
                    .and_then(memory_decay::age_from_timestamp);
                entries.push((current_entry.clone(), age));
            }
            current_timestamp = Some(line.to_string());
            current_entry = format!("{}\n", line);
        } else {
            current_entry.push_str(line);
            current_entry.push('\n');
        }
    }

    // Save last entry
    if !current_entry.trim().is_empty() {
        let age = current_timestamp
            .as_deref()
            .and_then(memory_decay::age_from_timestamp);
        entries.push((current_entry, age));
    }

    // If no entries found (file has no ## headers), treat entire file as one entry
    if entries.is_empty() && !content.trim().is_empty() {
        entries.push((format!("**{}**\n{}", filename, content), None));
    }

    entries
}

#[async_trait]
impl Tool for MemorySearchTool {
    fn name(&self) -> &str {
        "search_memory"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "search_memory".into(),
            description: "Search past memories, solutions, and error patterns using BM25 ranking with temporal decay. Use this BEFORE attempting to solve a problem to see if you've solved it before. Recent memories are ranked higher than old ones.".into(),
            input_schema: schema_object(
                json!({
                    "query": {
                        "type": "string",
                        "description": "What to search for (error message, topic, solution keyword)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Max results to return (default: 5)"
                    }
                }),
                &["query"],
            ),
        }
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult {
        let query = match input.get("query").and_then(|v| v.as_str()) {
            Some(q) => q,
            None => return ToolResult::error("Missing 'query' parameter".into()),
        };

        let limit = input
            .get("limit")
            .and_then(|v| v.as_i64())
            .unwrap_or(5) as usize;

        if !self.memory_dir.exists() {
            return ToolResult::success(format!("No memories found for: {}", query));
        }

        let entries = match fs::read_dir(&self.memory_dir) {
            Ok(e) => e,
            Err(_) => return ToolResult::success(format!("No memories found for: {}", query)),
        };

        // Collect all documents (entries from all files)
        let mut all_docs: Vec<(String, Vec<String>, String, Option<f64>)> = Vec::new(); // (text, tokens, filename, age)

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("md") {
                continue;
            }

            let filename = path
                .file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            if let Ok(content) = fs::read_to_string(&path) {
                let entries = parse_entries(&content, &filename);
                for (text, age) in entries {
                    let tokens = tokenize(&text);
                    if !tokens.is_empty() {
                        all_docs.push((text, tokens, filename.clone(), age));
                    }
                }
            }
        }

        if all_docs.is_empty() {
            return ToolResult::success(format!("No memories found for: {}", query));
        }

        let query_tokens = tokenize(query);
        if query_tokens.is_empty() {
            return ToolResult::success(format!("No memories found for: {}", query));
        }

        // Calculate document frequencies
        let n_docs = all_docs.len();
        let avg_doc_len = all_docs.iter().map(|(_, t, _, _)| t.len()).sum::<usize>() as f64
            / n_docs as f64;

        let mut doc_freq: HashMap<String, usize> = HashMap::new();
        for (_, tokens, _, _) in &all_docs {
            let unique: std::collections::HashSet<&str> =
                tokens.iter().map(|s| s.as_str()).collect();
            for term in unique {
                *doc_freq.entry(term.to_string()).or_insert(0) += 1;
            }
        }

        // Score all documents
        let mut scored: Vec<ScoredEntry> = all_docs
            .iter()
            .map(|(text, tokens, filename, age)| {
                let bm25 = bm25_score(tokens, &query_tokens, avg_doc_len, n_docs, &doc_freq);
                let half_life = memory_decay::half_life_for_category(filename);
                let decay = age
                    .map(|a| memory_decay::decay_score(a, half_life))
                    .unwrap_or(0.5); // Unknown age gets neutral score

                // Combined: BM25 * decay (decay is 0-1 multiplier)
                let combined = bm25 * (0.3 + 0.7 * decay); // 30% base + 70% decay-weighted

                // Truncate content for display
                let display = if text.len() > 300 {
                    format!("{}...", &text[..text.char_indices().take(300).last().map(|(i, _)| i).unwrap_or(300)])
                } else {
                    text.clone()
                };

                ScoredEntry {
                    filename: filename.clone(),
                    content: display,
                    bm25_score: bm25,
                    decay_score: decay,
                    combined_score: combined,
                }
            })
            .filter(|e| e.bm25_score > 0.0) // Only include matches
            .collect();

        // Sort by combined score descending
        scored.sort_by(|a, b| b.combined_score.partial_cmp(&a.combined_score).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);

        if scored.is_empty() {
            ToolResult::success(format!("No memories found for: {}", query))
        } else {
            let results: Vec<String> = scored
                .iter()
                .map(|e| {
                    format!(
                        "**{}.md** (relevance: {:.2}, freshness: {:.0}%)\n{}",
                        e.filename,
                        e.bm25_score,
                        e.decay_score * 100.0,
                        e.content.trim()
                    )
                })
                .collect();

            ToolResult::success(format!(
                "Found {} relevant memories:\n\n{}",
                results.len(),
                results.join("\n\n---\n\n")
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize() {
        let tokens = tokenize("Hello world, this is a test!");
        assert!(tokens.contains(&"hello".to_string()));
        assert!(tokens.contains(&"world".to_string()));
        assert!(tokens.contains(&"test".to_string()));
        assert!(!tokens.contains(&"a".to_string())); // too short
    }

    #[test]
    fn test_tokenize_with_underscores() {
        let tokens = tokenize("search_memory tool_name");
        assert!(tokens.contains(&"search_memory".to_string()));
        assert!(tokens.contains(&"tool_name".to_string()));
    }

    #[test]
    fn test_bm25_score_matching() {
        let doc = tokenize("rust programming language for systems");
        let query = tokenize("rust programming");
        let mut doc_freq = HashMap::new();
        doc_freq.insert("rust".to_string(), 1);
        doc_freq.insert("programming".to_string(), 1);

        let score = bm25_score(&doc, &query, 5.0, 3, &doc_freq);
        assert!(score > 0.0);
    }

    #[test]
    fn test_bm25_score_no_match() {
        let doc = tokenize("python machine learning");
        let query = tokenize("rust programming");
        let doc_freq = HashMap::new();

        let score = bm25_score(&doc, &query, 5.0, 3, &doc_freq);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_bm25_higher_tf_higher_score() {
        let doc1 = tokenize("rust rust rust programming");
        let doc2 = tokenize("rust programming python java");
        let query = tokenize("rust");
        let mut doc_freq = HashMap::new();
        doc_freq.insert("rust".to_string(), 2);

        let score1 = bm25_score(&doc1, &query, 4.0, 5, &doc_freq);
        let score2 = bm25_score(&doc2, &query, 4.0, 5, &doc_freq);
        assert!(score1 > score2, "Higher TF should give higher score");
    }

    #[test]
    fn test_parse_entries_with_timestamps() {
        let content = "## 2025-01-15 10:30:00 UTC\n\nFixed the scheduler bug.\n\n## 2025-02-01 14:00:00 UTC\n\nUpdated the config.\n";
        let entries = parse_entries(content, "solutions");
        assert_eq!(entries.len(), 2);
        assert!(entries[0].0.contains("scheduler"));
        assert!(entries[0].1.is_some()); // Should have age
        assert!(entries[1].0.contains("config"));
    }

    #[test]
    fn test_parse_entries_no_timestamps() {
        let content = "Some content without headers\nAnother line\n";
        let entries = parse_entries(content, "notes");
        assert_eq!(entries.len(), 1);
    }

    #[tokio::test]
    async fn test_memory_search_bm25() {
        let dir = std::env::temp_dir().join(format!("sandy_bm25_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();

        // Create test memory files with enough entries for good IDF
        std::fs::write(
            dir.join("solutions.md"),
            "## 2026-02-15 10:00:00 UTC\n\nFixed the scheduler by updating the cron expression from 5-field to 6-field format.\n\n## 2026-02-10 08:00:00 UTC\n\nResolved database connection timeout by increasing pool size to 10.\n\n## 2026-02-08 08:00:00 UTC\n\nImproved memory search with BM25 ranking algorithm for better results.\n\n## 2026-02-05 08:00:00 UTC\n\nAdded hook system for pre and post tool execution lifecycle.\n",
        ).unwrap();

        std::fs::write(
            dir.join("errors.md"),
            "## 2026-02-14 12:00:00 UTC\n\nError: webhook server failed to start on port 8080.\n\n## 2026-02-12 12:00:00 UTC\n\nError: LLM rate limit exceeded during high traffic period.\n",
        ).unwrap();

        let tool = MemorySearchTool::new(dir.clone());

        // Search for "scheduler" - unique term, should get good IDF
        let result = tool.execute(json!({"query": "scheduler cron"})).await;
        assert!(!result.is_error);
        assert!(result.content.contains("scheduler"));
        assert!(result.content.contains("relevant memories"));

        // Search for "BM25 ranking" - should find the memory search entry
        let result = tool.execute(json!({"query": "BM25 ranking"})).await;
        assert!(!result.is_error);
        assert!(result.content.contains("BM25"));

        // Search for nonexistent
        let result = tool.execute(json!({"query": "kubernetes deployment"})).await;
        assert!(result.content.contains("No memories found"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_memory_search_empty_dir() {
        let dir = std::env::temp_dir().join(format!("sandy_bm25_empty_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();

        let tool = MemorySearchTool::new(dir.clone());
        let result = tool.execute(json!({"query": "anything"})).await;
        assert!(result.content.contains("No memories found"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_memory_search_missing_query() {
        let tool = MemorySearchTool::new(PathBuf::from("/tmp"));
        let result = tool.execute(json!({})).await;
        assert!(result.is_error);
        assert!(result.content.contains("Missing"));
    }
}
