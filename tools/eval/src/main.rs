//! Reference-corpus evaluation harness for tract.
//!
//! Runs `Orchestrator::extract_from_html` against every committed fixture and
//! scores its markdown against the hand-curated `expected/<id>.md`. Writes a
//! per-bucket summary to `tests/corpus/baseline.json` and prints a table to
//! stderr.
//!
//! Bench-only in Phase A: no `--mode` flag, no live fetching. Live mode lands
//! in Phase C alongside the verifier.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};
use tract_profile::Profile;
use tractd::orchestrator::Orchestrator;

#[derive(Parser, Debug)]
#[command(name = "eval", about = "Run tract against the reference corpus")]
struct Cli {
    /// Restrict to one bucket (trivial / moderate / hard / adversarial).
    #[arg(long)]
    filter: Option<String>,
    /// Where the corpus lives. Defaults to ../../tests/corpus relative to the
    /// workspace root.
    #[arg(long)]
    corpus: Option<PathBuf>,
    /// Print extractor output + expected for a single entry, then exit.
    #[arg(long)]
    inspect: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Corpus {
    entries: Vec<Entry>,
}

#[derive(Debug, Deserialize)]
struct Entry {
    id: String,
    url: String,
    bucket: String,
    #[serde(default)]
    #[allow(dead_code)]
    expected_kind: String,
    #[serde(default)]
    #[allow(dead_code)]
    notes: String,
}

#[derive(Debug, Serialize)]
struct Report {
    ran_at: String,
    profile_name: String,
    buckets: BTreeMap<String, BucketSummary>,
    entries: Vec<EntryResult>,
}

#[derive(Debug, Serialize, Default, Clone)]
struct BucketSummary {
    count: usize,
    scored: usize,
    missing_fixture: usize,
    missing_expected: usize,
    extract_failed: usize,
    avg_similarity: f32,
    threshold: f32,
}

#[derive(Debug, Serialize)]
struct EntryResult {
    id: String,
    bucket: String,
    status: String,
    similarity: Option<f32>,
    text_recall: Option<f32>,
    text_precision: Option<f32>,
    passed: Option<bool>,
}

fn workspace_root() -> Result<PathBuf> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let p = Path::new(manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .context("locate workspace root")?
        .to_path_buf();
    Ok(p)
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = workspace_root()?;
    let corpus_dir = cli
        .corpus
        .clone()
        .unwrap_or_else(|| root.join("tests").join("corpus"));

    let corpus: Corpus = {
        let s = fs::read_to_string(corpus_dir.join("urls.toml"))
            .with_context(|| format!("read {}/urls.toml", corpus_dir.display()))?;
        toml::from_str(&s).context("parse urls.toml")?
    };

    let profile = Profile::default();

    if let Some(id) = cli.inspect.as_ref() {
        return inspect_one(&corpus, &corpus_dir, &profile, id);
    }

    let buckets_filter: Option<HashSet<String>> =
        cli.filter.as_ref().map(|b| HashSet::from([b.clone()]));

    let mut results = Vec::with_capacity(corpus.entries.len());
    let mut bucket_summaries: BTreeMap<String, BucketSummary> = BTreeMap::new();

    for entry in &corpus.entries {
        if let Some(f) = &buckets_filter {
            if !f.contains(&entry.bucket) {
                continue;
            }
        }
        let summary = bucket_summaries
            .entry(entry.bucket.clone())
            .or_default();
        summary.count += 1;
        summary.threshold = threshold_for(&entry.bucket, &profile);

        let fixture = corpus_dir.join("fixtures").join(format!("{}.html", entry.id));
        let expected = corpus_dir.join("expected").join(format!("{}.md", entry.id));

        if !fixture.exists() {
            summary.missing_fixture += 1;
            results.push(EntryResult {
                id: entry.id.clone(),
                bucket: entry.bucket.clone(),
                status: "missing-fixture".into(),
                similarity: None,
                text_recall: None,
                text_precision: None,
                passed: None,
            });
            continue;
        }
        if !expected.exists() {
            summary.missing_expected += 1;
            results.push(EntryResult {
                id: entry.id.clone(),
                bucket: entry.bucket.clone(),
                status: "missing-expected".into(),
                similarity: None,
                text_recall: None,
                text_precision: None,
                passed: None,
            });
            continue;
        }

        let html =
            fs::read_to_string(&fixture).with_context(|| format!("read {}", fixture.display()))?;
        let want = fs::read_to_string(&expected)
            .with_context(|| format!("read {}", expected.display()))?;

        let result = match Orchestrator::extract_from_html(&profile, &html, &entry.url) {
            Ok(r) => r,
            Err(e) => {
                summary.extract_failed += 1;
                results.push(EntryResult {
                    id: entry.id.clone(),
                    bucket: entry.bucket.clone(),
                    status: format!("extract-failed: {}", e.message),
                    similarity: None,
                    text_recall: None,
                    text_precision: None,
                    passed: None,
                });
                continue;
            }
        };

        let breakdown = similarity_breakdown(&result.markdown, &want);
        let passed = breakdown.overall >= summary.threshold;
        summary.scored += 1;
        // Running mean over scored entries.
        let n = summary.scored as f32;
        summary.avg_similarity =
            (summary.avg_similarity * (n - 1.0) + breakdown.overall) / n;

        results.push(EntryResult {
            id: entry.id.clone(),
            bucket: entry.bucket.clone(),
            status: "scored".into(),
            similarity: Some(breakdown.overall),
            text_recall: Some(breakdown.recall),
            text_precision: Some(breakdown.precision),
            passed: Some(passed),
        });
    }

    let report = Report {
        ran_at: chrono::Utc::now().to_rfc3339(),
        profile_name: profile.name.clone(),
        buckets: bucket_summaries.clone(),
        entries: results,
    };

    let baseline_path = corpus_dir.join("baseline.json");
    let json = serde_json::to_string_pretty(&report)?;
    fs::write(&baseline_path, &json)
        .with_context(|| format!("write {}", baseline_path.display()))?;

    print_summary(&report);

    // Phase A: only the trivial bucket blocks exit. Other buckets are
    // reported but not required.
    let trivial = bucket_summaries.get("trivial");
    let trivial_passes = match trivial {
        Some(s) if s.scored > 0 => s.avg_similarity >= s.threshold,
        _ => true, // no trivial entries scored -> don't block
    };

    if !trivial_passes {
        eprintln!(
            "FAIL: trivial bucket avg_similarity below threshold ({:.3} < {:.3})",
            trivial.map(|s| s.avg_similarity).unwrap_or(0.0),
            trivial.map(|s| s.threshold).unwrap_or(0.95),
        );
        std::process::exit(1);
    }
    Ok(())
}

fn print_summary(report: &Report) {
    eprintln!("eval @ {}  profile={}", report.ran_at, report.profile_name);
    eprintln!("{:-<78}", "");
    eprintln!(
        "{:<14}{:>7}{:>9}{:>11}{:>12}{:>12}{:>10}",
        "bucket", "count", "scored", "miss-fix", "miss-exp", "avg-sim", "threshold"
    );
    for (name, s) in &report.buckets {
        eprintln!(
            "{:<14}{:>7}{:>9}{:>11}{:>12}{:>12.3}{:>10.2}",
            name, s.count, s.scored, s.missing_fixture, s.missing_expected, s.avg_similarity, s.threshold
        );
    }
    eprintln!("{:-<78}", "");
    eprintln!("per-entry  similarity   recall  precision");
    for r in &report.entries {
        match (r.similarity, r.text_recall, r.text_precision, r.passed) {
            (Some(sim), Some(rec), Some(prec), Some(pass)) => {
                let mark = if pass { "PASS" } else { "FAIL" };
                eprintln!(
                    "  [{}] {:<28} {:.3}    {:.3}    {:.3}    ({})",
                    mark, r.id, sim, rec, prec, r.bucket
                );
            }
            _ => {
                eprintln!(
                    "  [{:>4}] {:<28}                            ({}; {})",
                    "----", r.id, r.bucket, r.status
                );
            }
        }
    }
}

fn inspect_one(corpus: &Corpus, corpus_dir: &Path, profile: &Profile, id: &str) -> Result<()> {
    let entry = corpus
        .entries
        .iter()
        .find(|e| e.id == id)
        .with_context(|| format!("no entry with id={id}"))?;
    let fixture = corpus_dir.join("fixtures").join(format!("{}.html", id));
    let expected_path = corpus_dir.join("expected").join(format!("{}.md", id));

    let html = fs::read_to_string(&fixture)
        .with_context(|| format!("read {}", fixture.display()))?;
    let want = fs::read_to_string(&expected_path)
        .with_context(|| format!("read {}", expected_path.display()))?;
    let got = Orchestrator::extract_from_html(profile, &html, &entry.url)
        .map_err(|e| anyhow::anyhow!("extract failed: {}", e.message))?;

    let b = similarity_breakdown(&got.markdown, &want);

    println!("=== entry: {} (bucket: {}) ===", id, entry.bucket);
    println!(
        "=== similarity: {:.3}   recall: {:.3}   precision: {:.3} ===",
        b.overall, b.recall, b.precision
    );
    println!("\n=== EXTRACTOR OUTPUT ({} chars) ===\n", got.markdown.len());
    println!("{}", got.markdown);
    println!("\n=== EXPECTED ({} chars) ===\n", want.len());
    println!("{}", want);
    Ok(())
}

fn threshold_for(bucket: &str, profile: &Profile) -> f32 {
    match bucket {
        "trivial" => profile.eval.trivial_threshold,
        "moderate" => profile.eval.moderate_threshold,
        "hard" => profile.eval.hard_threshold,
        _ => 0.0, // adversarial isn't scored on similarity in Phase A
    }
}

struct SimilarityBreakdown {
    overall: f32,
    precision: f32,
    recall: f32,
}

fn similarity_breakdown(actual: &str, expected: &str) -> SimilarityBreakdown {
    let a = normalize(actual);
    let e = normalize(expected);
    if a.is_empty() && e.is_empty() {
        return SimilarityBreakdown {
            overall: 1.0,
            precision: 1.0,
            recall: 1.0,
        };
    }
    let (precision, recall) = sentence_pr(&a, &e);
    let text_sim = if precision + recall == 0.0 {
        0.0
    } else {
        2.0 * precision * recall / (precision + recall)
    };
    let headings = jaccard(&headings_of(&a), &headings_of(&e));
    let list_match = count_match(count_lines_starting(&a, "- "), count_lines_starting(&e, "- "));
    let link_match = count_match(count_occurrences(&a, "]("), count_occurrences(&e, "]("));
    let overall = 0.6 * text_sim + 0.2 * headings + 0.1 * list_match + 0.1 * link_match;
    SimilarityBreakdown {
        overall,
        precision,
        recall,
    }
}

#[cfg(test)]
fn similarity(actual: &str, expected: &str) -> f32 {
    similarity_breakdown(actual, expected).overall
}

fn normalize(s: &str) -> String {
    // Collapse runs of whitespace, trim each line, drop empty lines. Markdown
    // semantic content stays; cosmetic line-breaks don't.
    s.lines()
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Sentence-level precision + recall.
///   - recall  = fraction of expected sentences found in actual
///     (catches: extractor truncated or missed content)
///   - precision = fraction of actual sentences found in expected
///     (catches: extractor leaked chrome / nav / footer)
///
/// Lets `expected/<id>.md` be partial — a 60-sentence Wikipedia spine still
/// produces a calibrated score against a 500-sentence article.
fn sentence_pr(actual: &str, expected: &str) -> (f32, f32) {
    let act_sents = sentences(actual);
    let exp_sents = sentences(expected);
    if act_sents.is_empty() && exp_sents.is_empty() {
        return (1.0, 1.0);
    }
    if act_sents.is_empty() {
        return (0.0, 0.0);
    }
    if exp_sents.is_empty() {
        return (0.0, 0.0);
    }
    let act_tokens: Vec<HashSet<String>> = act_sents.iter().map(|s| tokens(s)).collect();
    let exp_tokens: Vec<HashSet<String>> = exp_sents.iter().map(|s| tokens(s)).collect();

    let threshold = 0.6_f32;
    let recall_hits = exp_tokens
        .iter()
        .filter(|e| act_tokens.iter().any(|a| token_jaccard(a, e) >= threshold))
        .count();
    let precision_hits = act_tokens
        .iter()
        .filter(|a| exp_tokens.iter().any(|e| token_jaccard(a, e) >= threshold))
        .count();

    let recall = recall_hits as f32 / exp_tokens.len() as f32;
    let precision = precision_hits as f32 / act_tokens.len() as f32;
    (precision, recall)
}

fn sentences(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in s.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // Each heading, list item, table row counts as one sentence; prose
        // gets split at . ! ? followed by space.
        if line.starts_with('#')
            || line.starts_with("- ")
            || line.starts_with("* ")
            || line.starts_with('|')
        {
            out.push(line.to_string());
            continue;
        }
        let mut buf = String::new();
        let mut chars = line.chars().peekable();
        while let Some(c) = chars.next() {
            buf.push(c);
            if matches!(c, '.' | '!' | '?') {
                if let Some(&next) = chars.peek() {
                    if next == ' ' {
                        let t = buf.trim();
                        if t.split_whitespace().count() >= 3 {
                            out.push(t.to_string());
                        }
                        buf.clear();
                    }
                }
            }
        }
        let t = buf.trim();
        if t.split_whitespace().count() >= 3 {
            out.push(t.to_string());
        }
    }
    out
}

fn tokens(s: &str) -> HashSet<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 2)
        .map(|w| w.to_string())
        .collect()
}

fn token_jaccard(a: &HashSet<String>, b: &HashSet<String>) -> f32 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let inter = a.intersection(b).count() as f32;
    let union = a.union(b).count() as f32;
    if union == 0.0 {
        0.0
    } else {
        inter / union
    }
}

fn headings_of(s: &str) -> HashSet<String> {
    s.lines()
        .filter_map(|l| {
            let l = l.trim();
            if let Some(rest) = l.strip_prefix("# ") {
                Some(rest.to_string())
            } else if let Some(rest) = l.strip_prefix("## ") {
                Some(rest.to_string())
            } else if let Some(rest) = l.strip_prefix("### ") {
                Some(rest.to_string())
            } else {
                None
            }
        })
        .collect()
}

fn jaccard(a: &HashSet<String>, b: &HashSet<String>) -> f32 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let inter = a.intersection(b).count() as f32;
    let union = a.union(b).count() as f32;
    if union == 0.0 {
        1.0
    } else {
        inter / union
    }
}

fn count_lines_starting(s: &str, prefix: &str) -> usize {
    s.lines().filter(|l| l.trim_start().starts_with(prefix)).count()
}

fn count_occurrences(s: &str, needle: &str) -> usize {
    s.matches(needle).count()
}

fn count_match(a: usize, b: usize) -> f32 {
    let diff = (a as i64 - b as i64).unsigned_abs() as f32;
    let denom = b.max(1) as f32;
    (1.0 - (diff / denom).min(1.0)).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn similarity_identical_is_one() {
        assert!(similarity("# Hi\n\ntext", "# Hi\n\ntext") >= 0.999);
    }

    #[test]
    fn similarity_disjoint_is_low() {
        let s = similarity("# A\n\ntotally different content", "# B\n\nother words entirely");
        assert!(s < 0.7, "got {s}");
    }

    #[test]
    fn headings_extracted() {
        let h = headings_of("# One\n## Two\nbody\n### Three");
        assert!(h.contains("One"));
        assert!(h.contains("Two"));
        assert!(h.contains("Three"));
        assert_eq!(h.len(), 3);
    }
}
