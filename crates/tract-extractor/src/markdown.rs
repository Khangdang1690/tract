//! Walk an HTML subtree and emit GitHub-flavored markdown.
//!
//! Handles headings, paragraphs, lists (nested), links, emphasis, inline
//! code, code blocks, blockquotes, images, line breaks, and horizontal
//! rules. Skips scripts, styles, forms, embedded media, and navigational
//! containers. Tables are deferred to v0.2.

use ego_tree::NodeRef;
use scraper::{ElementRef, Node};

pub fn render(root: ElementRef<'_>) -> String {
    let mut em = MdEmitter::default();
    for child in root.children() {
        em.emit(child);
    }
    em.finalize()
}

#[derive(Default)]
struct MdEmitter {
    buf: String,
    list_stack: Vec<ListKind>,
}

enum ListKind {
    Ul,
    Ol(u32),
}

impl MdEmitter {
    fn emit(&mut self, node: NodeRef<'_, Node>) {
        match node.value() {
            Node::Text(t) => self.write_text(t),
            Node::Element(_) => self.emit_element(node),
            _ => {}
        }
    }

    fn write_text(&mut self, t: &str) {
        let collapsed = collapse_ws(t);
        if collapsed.is_empty() {
            return;
        }
        self.buf.push_str(&collapsed);
    }

    fn emit_children(&mut self, el: ElementRef<'_>) {
        for child in el.children() {
            self.emit(child);
        }
    }

    fn emit_element(&mut self, node: NodeRef<'_, Node>) {
        let Some(el) = ElementRef::wrap(node) else {
            return;
        };
        match el.value().name() {
            "h1" => self.heading(el, 1),
            "h2" => self.heading(el, 2),
            "h3" => self.heading(el, 3),
            "h4" => self.heading(el, 4),
            "h5" => self.heading(el, 5),
            "h6" => self.heading(el, 6),
            "p" => self.paragraph(el),
            "br" => self.buf.push_str("  \n"),
            "hr" => {
                self.ensure_blank();
                self.buf.push_str("---\n\n");
            }
            "ul" => {
                self.list_stack.push(ListKind::Ul);
                self.ensure_blank();
                self.emit_children(el);
                self.list_stack.pop();
                self.ensure_blank();
            }
            "ol" => {
                self.list_stack.push(ListKind::Ol(1));
                self.ensure_blank();
                self.emit_children(el);
                self.list_stack.pop();
                self.ensure_blank();
            }
            "li" => self.list_item(el),
            "a" => self.link(el),
            "strong" | "b" => self.wrap(el, "**", "**"),
            "em" | "i" => self.wrap(el, "*", "*"),
            "code" => {
                // <code> inside <pre> is rendered by the <pre> branch.
                if !el
                    .ancestors()
                    .any(|a| ElementRef::wrap(a).is_some_and(|e| e.value().name() == "pre"))
                {
                    let text = el.text().collect::<String>();
                    self.buf.push('`');
                    self.buf.push_str(&text);
                    self.buf.push('`');
                }
            }
            "pre" => self.code_block(el),
            "blockquote" => self.blockquote(el),
            "img" => self.image(el),
            "figure" => self.emit_children(el),
            "figcaption" => self.paragraph(el),
            // Strip noise wholesale.
            "script" | "style" | "noscript" | "iframe" | "svg" | "picture" | "video" | "audio"
            | "form" | "button" | "input" | "select" | "textarea" | "label" | "nav" | "aside"
            | "footer" | "header" | "menu" | "dialog" => {}
            // Tables: emit text content as plain paragraphs for MVP.
            "table" | "thead" | "tbody" | "tfoot" | "tr" | "td" | "th" | "caption" => {
                self.emit_children(el);
            }
            // Everything else: just walk through (divs, spans, sections, etc.).
            _ => self.emit_children(el),
        }
    }

    fn heading(&mut self, el: ElementRef<'_>, level: usize) {
        self.ensure_blank();
        for _ in 0..level {
            self.buf.push('#');
        }
        self.buf.push(' ');
        let mut inner = MdEmitter::default();
        inner.emit_children(el);
        let s = inner.finalize();
        self.buf.push_str(s.trim());
        self.buf.push_str("\n\n");
    }

    fn paragraph(&mut self, el: ElementRef<'_>) {
        self.ensure_blank();
        let mut inner = MdEmitter::default();
        inner.emit_children(el);
        let s = inner.finalize();
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return;
        }
        self.buf.push_str(trimmed);
        self.buf.push_str("\n\n");
    }

    fn list_item(&mut self, el: ElementRef<'_>) {
        // Determine bullet from innermost list.
        let depth = self.list_stack.len().saturating_sub(1);
        let indent = "  ".repeat(depth);
        let bullet = match self.list_stack.last_mut() {
            Some(ListKind::Ol(counter)) => {
                let s = format!("{counter}. ");
                *counter += 1;
                s
            }
            _ => "- ".to_string(),
        };

        // Render inner content into a sub-buffer so nested lists work.
        let mut inner = MdEmitter {
            buf: String::new(),
            list_stack: std::mem::take(&mut self.list_stack),
        };
        inner.emit_children(el);
        self.list_stack = std::mem::take(&mut inner.list_stack);
        let rendered = inner.finalize();

        // Trim trailing newlines, indent continuation lines.
        let trimmed = rendered.trim_end();
        let mut lines = trimmed.lines();
        let Some(first) = lines.next() else {
            return;
        };
        self.buf.push_str(&indent);
        self.buf.push_str(&bullet);
        self.buf.push_str(first.trim_start());
        self.buf.push('\n');
        for line in lines {
            if line.is_empty() {
                self.buf.push('\n');
            } else {
                self.buf.push_str(&indent);
                self.buf.push_str("  ");
                self.buf.push_str(line);
                self.buf.push('\n');
            }
        }
    }

    fn link(&mut self, el: ElementRef<'_>) {
        let href = el.value().attr("href").unwrap_or("");
        let mut inner = MdEmitter::default();
        inner.emit_children(el);
        let text = inner.finalize();
        let text = text.trim();
        if text.is_empty() && href.is_empty() {
            return;
        }
        if href.is_empty() {
            self.buf.push_str(text);
            return;
        }
        if text.is_empty() {
            self.buf.push_str(href);
            return;
        }
        self.buf.push('[');
        self.buf.push_str(text);
        self.buf.push(']');
        self.buf.push('(');
        self.buf.push_str(href);
        self.buf.push(')');
    }

    fn wrap(&mut self, el: ElementRef<'_>, open: &str, close: &str) {
        let mut inner = MdEmitter::default();
        inner.emit_children(el);
        let text = inner.finalize();
        let text = text.trim();
        if text.is_empty() {
            return;
        }
        self.buf.push_str(open);
        self.buf.push_str(text);
        self.buf.push_str(close);
    }

    fn code_block(&mut self, el: ElementRef<'_>) {
        // Best-effort language detection from a <code class="language-xxx"> child.
        let mut lang = "";
        if let Some(code_child) = el
            .children()
            .find_map(|c| ElementRef::wrap(c).filter(|e| e.value().name() == "code"))
        {
            if let Some(class) = code_child.value().attr("class") {
                for tok in class.split_ascii_whitespace() {
                    if let Some(rest) = tok.strip_prefix("language-") {
                        lang = rest;
                        break;
                    }
                }
            }
        }
        let raw = el.text().collect::<String>();
        let trimmed = raw.trim_end_matches('\n');
        self.ensure_blank();
        self.buf.push_str("```");
        self.buf.push_str(lang);
        self.buf.push('\n');
        self.buf.push_str(trimmed);
        self.buf.push('\n');
        self.buf.push_str("```\n\n");
    }

    fn blockquote(&mut self, el: ElementRef<'_>) {
        let mut inner = MdEmitter::default();
        inner.emit_children(el);
        let rendered = inner.finalize();
        self.ensure_blank();
        for line in rendered.trim_end().lines() {
            if line.is_empty() {
                self.buf.push_str(">\n");
            } else {
                self.buf.push_str("> ");
                self.buf.push_str(line);
                self.buf.push('\n');
            }
        }
        self.buf.push('\n');
    }

    fn image(&mut self, el: ElementRef<'_>) {
        let src = el.value().attr("src").unwrap_or("");
        let alt = el.value().attr("alt").unwrap_or("");
        if src.is_empty() {
            return;
        }
        self.buf.push_str("![");
        self.buf.push_str(alt);
        self.buf.push_str("](");
        self.buf.push_str(src);
        self.buf.push(')');
    }

    fn ensure_blank(&mut self) {
        if self.buf.is_empty() {
            return;
        }
        // Trim trailing whitespace down to at most two newlines.
        while self.buf.ends_with(' ') || self.buf.ends_with('\t') {
            self.buf.pop();
        }
        let trailing_nl = self.buf.chars().rev().take_while(|c| *c == '\n').count();
        match trailing_nl {
            0 => self.buf.push_str("\n\n"),
            1 => self.buf.push('\n'),
            _ => {}
        }
    }

    fn finalize(mut self) -> String {
        // Collapse 3+ consecutive newlines into 2.
        let mut out = String::with_capacity(self.buf.len());
        let mut nl_run = 0;
        for c in self.buf.drain(..) {
            if c == '\n' {
                nl_run += 1;
                if nl_run <= 2 {
                    out.push(c);
                }
            } else {
                nl_run = 0;
                out.push(c);
            }
        }
        out.trim().to_string()
    }
}

/// Collapse runs of whitespace into single spaces. Tabs and newlines become
/// spaces; this matches HTML's whitespace handling for non-`<pre>` content.
fn collapse_ws(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_space = false;
    for c in s.chars() {
        if c.is_whitespace() {
            if !last_space {
                out.push(' ');
                last_space = true;
            }
        } else {
            out.push(c);
            last_space = false;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use scraper::Html;

    fn run(html: &str) -> String {
        let doc = Html::parse_document(html);
        let body = doc
            .select(&scraper::Selector::parse("body").unwrap())
            .next()
            .unwrap();
        render(body)
    }

    #[test]
    fn heading_and_paragraph() {
        let md = run("<body><h1>Title</h1><p>Hello world.</p></body>");
        assert_eq!(md, "# Title\n\nHello world.");
    }

    #[test]
    fn emphasis_and_link() {
        let md = run(
            r#"<body><p>This is <strong>bold</strong> and <em>italic</em> and <a href="https://x.test/">a link</a>.</p></body>"#,
        );
        assert_eq!(
            md,
            "This is **bold** and *italic* and [a link](https://x.test/)."
        );
    }

    #[test]
    fn inline_code() {
        let md = run("<body><p>use <code>println!()</code> here.</p></body>");
        assert_eq!(md, "use `println!()` here.");
    }

    #[test]
    fn unordered_list() {
        let md = run("<body><ul><li>one</li><li>two</li></ul></body>");
        assert_eq!(md, "- one\n- two");
    }

    #[test]
    fn ordered_list_increments() {
        let md = run("<body><ol><li>first</li><li>second</li><li>third</li></ol></body>");
        assert_eq!(md, "1. first\n2. second\n3. third");
    }

    #[test]
    fn nested_list() {
        let md = run(
            "<body><ul><li>outer<ul><li>inner-a</li><li>inner-b</li></ul></li><li>next</li></ul></body>",
        );
        assert!(md.contains("- outer"), "got: {md}");
        assert!(md.contains("  - inner-a"), "got: {md}");
        assert!(md.contains("  - inner-b"), "got: {md}");
        assert!(md.contains("- next"), "got: {md}");
    }

    #[test]
    fn blockquote() {
        let md = run("<body><blockquote><p>To be or not to be.</p></blockquote></body>");
        assert_eq!(md, "> To be or not to be.");
    }

    #[test]
    fn code_block_with_language() {
        let md = run(r#"<body><pre><code class="language-rust">fn main() {}
</code></pre></body>"#);
        assert_eq!(md, "```rust\nfn main() {}\n```");
    }

    #[test]
    fn image_with_alt() {
        let md = run(r#"<body><p>see <img src="/cat.png" alt="a cat"></p></body>"#);
        assert_eq!(md, "see ![a cat](/cat.png)");
    }

    #[test]
    fn strips_script_and_style() {
        let md = run(
            "<body><script>alert(1)</script><style>p{color:red}</style><p>only this</p></body>",
        );
        assert_eq!(md, "only this");
    }

    #[test]
    fn collapses_whitespace() {
        let md = run("<body><p>foo\n   bar\t  baz</p></body>");
        assert_eq!(md, "foo bar baz");
    }

    #[test]
    fn hr_separates_blocks() {
        let md = run("<body><p>one</p><hr><p>two</p></body>");
        assert_eq!(md, "one\n\n---\n\ntwo");
    }
}
