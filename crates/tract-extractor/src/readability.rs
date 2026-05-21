//! Pick the element most likely to contain the article body.
//!
//! MVP approach: try a curated list of structural selectors that catch ~80%
//! of well-marked-up sites; fall back to a scored search over `<div>`s when
//! nothing structural matches; final fallback is `<body>`.

use scraper::{ElementRef, Html, Selector};

/// Selectors that, when present and substantive, almost certainly mark the
/// main article body. Order matters: most specific first.
const CANDIDATE_SELECTORS: &[&str] = &[
    "article",
    "main article",
    "main",
    "[role=main]",
    ".article-body",
    ".article__body",
    ".articleBody",
    ".post-content",
    ".post__content",
    ".entry-content",
    ".entry__content",
    ".story-body",
    ".content-body",
    "#content",
    "#main-content",
    "#bodyContent",
    "#mw-content-text",
];

/// Minimum visible text length for a structural candidate to be accepted
/// without falling through to scoring.
const MIN_TEXT_LEN: usize = 200;

/// Find the best content container in the document.
pub fn find_main_content(doc: &Html) -> ElementRef<'_> {
    for sel_str in CANDIDATE_SELECTORS {
        if let Some(el) = first_matching(doc, sel_str) {
            if visible_text_len(el) >= MIN_TEXT_LEN {
                return el;
            }
        }
    }

    if let Some(el) = score_best_div(doc) {
        return el;
    }

    body_or_root(doc)
}

fn first_matching<'a>(doc: &'a Html, sel: &str) -> Option<ElementRef<'a>> {
    let parsed = Selector::parse(sel).ok()?;
    doc.select(&parsed).next()
}

fn body_or_root(doc: &Html) -> ElementRef<'_> {
    if let Some(body) = first_matching(doc, "body") {
        return body;
    }
    doc.root_element()
}

/// Pick the highest-scoring `<div>` if no structural selector matched.
/// Score combines text length, paragraph count, and class/id signals;
/// link density damps the result.
fn score_best_div(doc: &Html) -> Option<ElementRef<'_>> {
    let sel = Selector::parse("div, section").ok()?;
    let mut best: Option<(f32, ElementRef<'_>)> = None;
    for el in doc.select(&sel) {
        let score = score(el);
        if score < 200.0 {
            continue;
        }
        match best {
            Some((s, _)) if s >= score => {}
            _ => best = Some((score, el)),
        }
    }
    best.map(|(_, el)| el)
}

fn score(el: ElementRef<'_>) -> f32 {
    let total = visible_text_len(el) as f32;
    if total < 50.0 {
        return 0.0;
    }
    let link = link_text_len(el) as f32;
    let link_density = (link / total).min(1.0);

    let para_count = count_descendants(el, "p") as f32;
    let head_count = (count_descendants(el, "h1")
        + count_descendants(el, "h2")
        + count_descendants(el, "h3")) as f32;

    let bonus = class_id_bonus(el);

    let base = total + para_count * 80.0 + head_count * 40.0 + bonus;
    base * (1.0 - link_density.min(0.9))
}

fn class_id_bonus(el: ElementRef<'_>) -> f32 {
    let mut s = String::new();
    if let Some(c) = el.value().attr("class") {
        s.push_str(c);
        s.push(' ');
    }
    if let Some(i) = el.value().attr("id") {
        s.push_str(i);
    }
    let s = s.to_ascii_lowercase();
    let mut bonus: f32 = 0.0;
    for tok in &[
        "article", "content", "post", "story", "entry", "body", "main",
    ] {
        if s.contains(tok) {
            bonus += 80.0;
        }
    }
    for tok in &[
        "nav", "footer", "sidebar", "aside", "comment", "promo", "advert", "cookie", "modal",
        "popup", "menu", "share", "social", "related",
    ] {
        if s.contains(tok) {
            bonus -= 100.0;
        }
    }
    bonus
}

fn visible_text_len(el: ElementRef<'_>) -> usize {
    let mut total = 0;
    for descendant in el.descendants() {
        if let Some(d_el) = ElementRef::wrap(descendant) {
            let name = d_el.value().name();
            if is_skipped_for_text(name) {
                continue;
            }
        }
        if let scraper::Node::Text(t) = descendant.value() {
            total += t.chars().filter(|c| !c.is_whitespace()).count();
            total += t.matches(' ').count(); // keep some weight for spaces between words
        }
    }
    total
}

fn link_text_len(el: ElementRef<'_>) -> usize {
    let sel = match Selector::parse("a") {
        Ok(s) => s,
        Err(_) => return 0,
    };
    let mut total = 0;
    for a in el.select(&sel) {
        for descendant in a.descendants() {
            if let scraper::Node::Text(t) = descendant.value() {
                total += t.chars().filter(|c| !c.is_whitespace()).count();
            }
        }
    }
    total
}

fn count_descendants(el: ElementRef<'_>, tag: &str) -> usize {
    Selector::parse(tag)
        .ok()
        .map(|s| el.select(&s).count())
        .unwrap_or(0)
}

fn is_skipped_for_text(tag: &str) -> bool {
    matches!(
        tag,
        "script"
            | "style"
            | "noscript"
            | "iframe"
            | "svg"
            | "picture"
            | "video"
            | "audio"
            | "form"
            | "button"
            | "input"
            | "select"
            | "textarea"
            | "nav"
            | "aside"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_article_over_nav() {
        let html = r#"
            <html><body>
                <nav>home about contact</nav>
                <article>
                    <h1>Hello</h1>
                    <p>This is a long paragraph with lots of words to make it count as content.
                       It needs to be substantial enough to clear the threshold.</p>
                    <p>A second paragraph for good measure.</p>
                </article>
                <footer>copyright</footer>
            </body></html>
        "#;
        let doc = Html::parse_document(html);
        let main = find_main_content(&doc);
        assert_eq!(main.value().name(), "article");
    }

    #[test]
    fn falls_back_to_body_for_trivial_doc() {
        let html = "<html><body><p>hi</p></body></html>";
        let doc = Html::parse_document(html);
        let main = find_main_content(&doc);
        assert_eq!(main.value().name(), "body");
    }

    #[test]
    fn class_bonus_lifts_post_content() {
        let html = r#"
            <html><body>
                <div class="sidebar">
                    <p>sidebar lorem ipsum dolor sit amet sidebar lorem ipsum dolor sit amet
                       sidebar lorem ipsum dolor sit amet sidebar lorem ipsum dolor sit amet</p>
                </div>
                <div class="post-content">
                    <p>This is the real article content paragraph with substantial text.</p>
                </div>
            </body></html>
        "#;
        let doc = Html::parse_document(html);
        let main = find_main_content(&doc);
        assert_eq!(
            main.value().attr("class"),
            Some("post-content"),
            "should match .post-content, got name={}",
            main.value().name()
        );
    }
}
