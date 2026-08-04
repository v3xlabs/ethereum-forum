use std::sync::LazyLock;

use htmd::HtmlToMarkdown;
use regex::Regex;

/// Discourse renders quoted posts as `<aside class="quote" data-username=…>`
/// wrapping a `div.title` of avatar and link chrome plus the quoted
/// `blockquote`. The attributes carry who and where, which the chrome does not.
static QUOTE_ASIDE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)<aside\b[^>]*\bclass="[^"]*\bquote\b[^"]*"[^>]*>"#).unwrap()
});
static ASIDE_ATTR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?is)\bdata-(username|topic|post)="([^"]*)""#).unwrap());
/// Avatar and permalink chrome inside a quote header.
static QUOTE_TITLE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)<div\b[^>]*\bclass="[^"]*\btitle\b[^"]*"[^>]*>.*?</div>"#).unwrap()
});
/// Upload chrome: the file name and "1448×1086 146 KB" dimension labels.
static UPLOAD_CHROME: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?is)<span\b[^>]*\bclass="[^"]*\b(filename|informations)\b[^"]*"[^>]*>.*?</span>"#,
    )
    .unwrap()
});
static BLANK_LINES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\n{3,}").unwrap());

static CONVERTER: LazyLock<HtmlToMarkdown> = LazyLock::new(|| {
    HtmlToMarkdown::builder()
        .skip_tags(vec!["svg", "script", "style", "noscript", "button"])
        .build()
});

/// Renders Discourse `cooked` HTML as Markdown for model input.
///
/// `cooked` is rendered HTML, not the author's markdown, so it carries real
/// structure the summarizer needs — code blocks, tables, links, lists and
/// quoted posts. Stripping tags would discard all of it, so this converts
/// rather than strips, and only drops rendering chrome (icons, avatars, upload
/// dimension labels).
pub fn cooked_to_markdown(cooked: &str) -> String {
    let prepared = QUOTE_TITLE.replace_all(cooked, "");
    let prepared = UPLOAD_CHROME.replace_all(&prepared, "");
    let prepared = QUOTE_ASIDE.replace_all(&prepared, |caps: &regex::Captures| {
        let tag = &caps[0];
        let mut username = None;
        let mut topic = None;
        let mut post = None;
        for attr in ASIDE_ATTR.captures_iter(tag) {
            let value = attr[2].to_string();
            match &attr[1] {
                "username" => username = Some(value),
                "topic" => topic = Some(value),
                _ => post = Some(value),
            }
        }
        match (username, topic, post) {
            (Some(user), Some(topic), Some(post)) => {
                format!("<p>quoting @{user} (topic {topic}, post {post}):</p>")
            }
            (Some(user), _, _) => format!("<p>quoting @{user}:</p>"),
            _ => String::new(),
        }
    });

    let markdown = CONVERTER
        .convert(&prepared)
        .unwrap_or_else(|_| prepared.to_string());
    BLANK_LINES.replace_all(markdown.trim(), "\n\n").into_owned()
}

#[cfg(test)]
mod tests {
    use super::cooked_to_markdown;

    #[test]
    fn code_blocks_survive_with_entities_decoded() {
        let html = r#"<pre><code class="lang-auto">mapping(bytes32 =&gt; address) public signerOf;</code></pre>"#;
        let out = cooked_to_markdown(html);
        assert!(out.contains("mapping(bytes32 => address) public signerOf;"));
        assert!(out.contains("```"));
    }

    #[test]
    fn links_keep_their_target() {
        let html = r#"<p>See <a href="https://eips.ethereum.org/EIPS/eip-1559">EIP-1559</a>.</p>"#;
        let out = cooked_to_markdown(html);
        assert!(out.contains("[EIP-1559](https://eips.ethereum.org/EIPS/eip-1559)"));
    }

    #[test]
    fn tables_are_preserved() {
        let html = r#"<table><thead><tr><th>Field</th><th>Type</th></tr></thead>
            <tbody><tr><td>tokenId</td><td>uint256</td></tr></tbody></table>"#;
        let out = cooked_to_markdown(html);
        assert!(out.contains("tokenId"));
        assert!(out.contains("uint256"));
        assert!(out.contains('|'));
    }

    #[test]
    fn lists_are_preserved() {
        let html = "<ul><li>first point</li><li>second point</li></ul>";
        let out = cooked_to_markdown(html);
        assert!(out.contains("first point"));
        assert!(out.contains("second point"));
        assert!(out.contains("* "));
    }

    #[test]
    fn quoted_posts_keep_their_attribution() {
        let html = r#"<aside class="quote no-group" data-username="atiselsts" data-post="5" data-topic="25543">
            <div class="title"><img src="/avatar.png"><a href="/t/25543/5">link</a></div>
            <blockquote><p>the original claim</p></blockquote></aside>"#;
        let out = cooked_to_markdown(html);
        assert!(out.contains("@atiselsts"));
        assert!(out.contains("topic 25543"));
        assert!(out.contains("post 5"));
        assert!(out.contains("the original claim"));
        assert!(!out.contains("avatar.png"));
    }

    #[test]
    fn rendering_chrome_is_dropped() {
        let html = r##"<p>see image</p><svg class="fa d-icon"><use href="#far-image"></use></svg>
            <span class="filename">upload_64379b06</span><span class="informations">1448×1086 146 KB</span>"##;
        let out = cooked_to_markdown(html);
        assert!(out.contains("see image"));
        assert!(!out.contains("far-image"));
        assert!(!out.contains("upload_64379b06"));
        assert!(!out.contains("146 KB"));
    }

    #[test]
    fn images_keep_their_reference() {
        let html = r#"<p><img src="/uploads/diagram.png" alt="state machine"></p>"#;
        let out = cooked_to_markdown(html);
        assert!(out.contains("state machine"));
    }
}
