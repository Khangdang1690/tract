//! Pick the element most likely to contain the article body.
//!
//! Try a curated list of structural selectors that catch ~80% of well-marked-up
//! sites; fall back to a scored search over `<div>`s when nothing structural
//! matches; final fallback is `<body>`. All tuning values come from
//! `ExtractorProfile`.

use scraper::{ElementRef, Html, Selector};
use tract_profile::ExtractorProfile;

/// Find the best content container in the document.
pub fn find_main_content<'a>(doc: &'a Html, profile: &ExtractorProfile) -> ElementRef<'a> {
    for sel_str in &profile.candidate_selectors {
        if let Some(el) = first_matching(doc, sel_str) {
            if visible_text_len(el) >= profile.min_text_len {
                return el;
            }
        }
    }

    if let Some(el) = score_best_div(doc, profile) {
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
fn score_best_div<'a>(doc: &'a Html, profile: &ExtractorProfile) -> Option<ElementRef<'a>> {
    let sel = Selector::parse("div, section").ok()?;
    let mut best: Option<(f32, ElementRef<'a>)> = None;
    for el in doc.select(&sel) {
        let score = score(el, profile);
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

fn score(el: ElementRef<'_>, profile: &ExtractorProfile) -> f32 {
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

    let bonus = class_id_bonus(el, profile);

    let base = total + para_count * profile.paragraph_weight + head_count * profile.heading_weight + bonus;
    base * (1.0 - link_density.min(profile.link_density_cap))
}

fn class_id_bonus(el: ElementRef<'_>, profile: &ExtractorProfile) -> f32 {
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
    for tok in &profile.class_id_positive_tokens {
        if s.contains(tok.as_str()) {
            bonus += profile.positive_token_bonus;
        }
    }
    for tok in &profile.class_id_negative_tokens {
        if s.contains(tok.as_str()) {
            bonus += profile.negative_token_penalty;
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
    use tract_profile::Profile;

    fn profile() -> ExtractorProfile {
        Profile::default().extractor
    }

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
        let main = find_main_content(&doc, &profile());
        assert_eq!(main.value().name(), "article");
    }

    #[test]
    fn falls_back_to_body_for_trivial_doc() {
        let html = "<html><body><p>hi</p></body></html>";
        let doc = Html::parse_document(html);
        let main = find_main_content(&doc, &profile());
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
        let main = find_main_content(&doc, &profile());
        assert_eq!(
            main.value().attr("class"),
            Some("post-content"),
            "should match .post-content, got name={}",
            main.value().name()
        );
    }
}
