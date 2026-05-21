//! HTML → readable markdown.

mod markdown;
mod readability;

pub use readability::find_main_content;

use scraper::{Html, Selector};

#[derive(Debug, Clone)]
pub struct Extracted {
    pub title: Option<String>,
    pub markdown: String,
    pub language: Option<String>,
    pub byline: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ExtractError {
    #[error("empty document")]
    Empty,
}

/// Parse `html` and return a `Extracted` with the article body as markdown.
pub fn extract(html: &str, _final_url: &str) -> Result<Extracted, ExtractError> {
    if html.trim().is_empty() {
        return Err(ExtractError::Empty);
    }
    let doc = Html::parse_document(html);

    let main = find_main_content(&doc);
    let markdown = markdown::render(main);

    let title = doc_title(&doc).or_else(|| first_heading(main));
    let language = lang_of(&doc);
    let byline = byline_of(&doc);

    Ok(Extracted {
        title,
        markdown,
        language,
        byline,
    })
}

fn doc_title(doc: &Html) -> Option<String> {
    let sel = Selector::parse("title").ok()?;
    let raw = doc.select(&sel).next()?.text().collect::<String>();
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn first_heading(el: scraper::ElementRef<'_>) -> Option<String> {
    let sel = Selector::parse("h1, h2").ok()?;
    let raw = el.select(&sel).next()?.text().collect::<String>();
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn lang_of(doc: &Html) -> Option<String> {
    let sel = Selector::parse("html").ok()?;
    doc.select(&sel)
        .next()?
        .value()
        .attr("lang")
        .map(|s| s.to_string())
}

fn byline_of(doc: &Html) -> Option<String> {
    let sel = Selector::parse(r#"meta[name="author"]"#).ok()?;
    let m = doc.select(&sel).next()?;
    m.value().attr("content").map(|s| s.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_extract_smoke() {
        let html = r#"<!doctype html>
            <html lang="en">
            <head>
                <title>Example Article</title>
                <meta name="author" content="Jane Doe">
            </head>
            <body>
                <nav>home</nav>
                <article>
                    <h1>Example Article</h1>
                    <p>Paragraph one is informative and long enough to count.</p>
                    <p>Paragraph two has <strong>bold</strong> and <em>italic</em>.</p>
                    <ul><li>a</li><li>b</li></ul>
                </article>
                <footer>copyright</footer>
            </body></html>"#;
        let out = extract(html, "https://x.test/article").unwrap();
        assert_eq!(out.title.as_deref(), Some("Example Article"));
        assert_eq!(out.language.as_deref(), Some("en"));
        assert_eq!(out.byline.as_deref(), Some("Jane Doe"));
        assert!(
            out.markdown.contains("# Example Article"),
            "md: {}",
            out.markdown
        );
        assert!(out.markdown.contains("**bold**"), "md: {}", out.markdown);
        assert!(out.markdown.contains("- a"), "md: {}", out.markdown);
        assert!(
            !out.markdown.contains("home"),
            "nav leaked: {}",
            out.markdown
        );
        assert!(
            !out.markdown.contains("copyright"),
            "footer leaked: {}",
            out.markdown
        );
    }

    #[test]
    fn empty_html_errors() {
        assert!(matches!(extract("", "x"), Err(ExtractError::Empty)));
    }
}
