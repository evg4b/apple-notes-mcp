//! Notes stores every line as its own `<div>`, often with inline styles, so the
//! markup is usually a larger share of a body than the words are. That is why
//! plain text is the default body format.

/// Tags that end the current line. Everything else is dropped silently; Notes
/// bodies carry no scripts or styles whose text would need suppressing.
const LINE_BREAKING_TAGS: &[&str] = &[
    "br",
    "div",
    "p",
    "li",
    "ul",
    "ol",
    "tr",
    "table",
    "blockquote",
    "pre",
    "hr",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
];

/// Strip the markup out of an HTML body, keeping its line structure.
pub fn to_plain_text(html: &str) -> String {
    let mut out = String::with_capacity(html.len() / 2);
    let mut rest = html;

    loop {
        let Some(open) = rest.find('<') else {
            push_decoded(&mut out, rest);
            break;
        };
        push_decoded(&mut out, &rest[..open]);

        let after_open = &rest[open + 1..];
        let Some(close) = after_open.find('>') else {
            // An unmatched '<' is literal text.
            push_decoded(&mut out, &rest[open..]);
            break;
        };
        if breaks_line(&after_open[..close]) {
            push_line_break(&mut out);
        }
        rest = &after_open[close + 1..];
    }

    tidy(out)
}

/// Does this tag body (the text between `<` and `>`) end the current line?
fn breaks_line(tag: &str) -> bool {
    let name = tag
        .trim_start_matches('/')
        .split(|c: char| c.is_whitespace() || c == '/')
        .next()
        .unwrap_or_default();
    LINE_BREAKING_TAGS
        .iter()
        .any(|known| known.eq_ignore_ascii_case(name))
}

fn push_line_break(out: &mut String) {
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
}

/// Append `text`, resolving the HTML entities Notes actually emits.
fn push_decoded(out: &mut String, text: &str) {
    let mut rest = text;
    loop {
        let Some(amp) = rest.find('&') else {
            out.push_str(rest);
            return;
        };
        out.push_str(&rest[..amp]);

        let after_amp = &rest[amp + 1..];
        // Entities are short; a '&' with no nearby ';' is a literal ampersand.
        match after_amp
            .char_indices()
            .take_while(|(i, _)| *i < 12)
            .find(|(_, c)| *c == ';')
            .map(|(i, _)| i)
            .and_then(|end| decode_entity(&after_amp[..end]).map(|ch| (ch, end)))
        {
            Some((decoded, end)) => {
                out.push(decoded);
                rest = &after_amp[end + 1..];
            }
            None => {
                out.push('&');
                rest = after_amp;
            }
        }
    }
}

/// Resolve the body of an entity, meaning whatever sits between `&` and `;`.
fn decode_entity(body: &str) -> Option<char> {
    match body {
        "amp" => return Some('&'),
        "lt" => return Some('<'),
        "gt" => return Some('>'),
        "quot" => return Some('"'),
        "apos" => return Some('\''),
        "nbsp" => return Some(' '),
        _ => {}
    }
    let digits = body.strip_prefix('#')?;
    let code = match digits.strip_prefix(['x', 'X']) {
        Some(hex) => u32::from_str_radix(hex, 16).ok()?,
        None => digits.parse().ok()?,
    };
    char::from_u32(code)
}

/// Drop trailing spaces, collapse runs of blank lines, and trim the ends.
fn tidy(text: String) -> String {
    let mut out = String::with_capacity(text.len());
    let mut blank_run = 0;
    for line in text.lines() {
        let line = line.trim_end();
        if line.is_empty() {
            blank_run += 1;
            if blank_run > 1 || out.is_empty() {
                continue;
            }
        } else {
            blank_run = 0;
        }
        out.push_str(line);
        out.push('\n');
    }
    while out.ends_with('\n') {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_passes_through() {
        assert_eq!(to_plain_text("just words"), "just words");
    }

    #[test]
    fn strips_tags_keeps_text() {
        assert_eq!(to_plain_text("<b>Hello</b> <i>world</i>"), "Hello world");
    }

    #[test]
    fn div_starts_a_new_line() {
        assert_eq!(
            to_plain_text("<div>one</div><div>two</div><div>three</div>"),
            "one\ntwo\nthree"
        );
    }

    #[test]
    fn br_breaks_the_line() {
        assert_eq!(to_plain_text("one<br>two"), "one\ntwo");
    }

    #[test]
    fn list_items_get_their_own_lines() {
        assert_eq!(
            to_plain_text("<ul><li>milk</li><li>eggs</li></ul>"),
            "milk\neggs"
        );
    }

    #[test]
    fn attributes_are_dropped() {
        assert_eq!(
            to_plain_text(r#"<div style="font-family: 'Helvetica'; color: #ff0000">red</div>"#),
            "red"
        );
    }

    #[test]
    fn tag_names_match_any_case() {
        assert_eq!(to_plain_text("<DIV>one</DIV><BR/>two"), "one\ntwo");
    }

    #[test]
    fn self_closing_br_breaks_the_line() {
        assert_eq!(to_plain_text("one<br />two"), "one\ntwo");
    }

    #[test]
    fn inline_tags_do_not_break_the_line() {
        assert_eq!(to_plain_text("a <span>b</span> c"), "a b c");
    }

    #[test]
    fn blank_line_runs_collapse() {
        assert_eq!(
            to_plain_text("<div>one</div><div><br></div><div><br></div><div>two</div>"),
            "one\ntwo"
        );
    }

    #[test]
    fn outer_blank_lines_are_trimmed() {
        assert_eq!(
            to_plain_text("<div><br></div><div>body</div><br><br>"),
            "body"
        );
    }

    #[test]
    fn named_entities_are_decoded() {
        assert_eq!(
            to_plain_text("Ben &amp; Jerry&apos;s &lt;tag&gt; &quot;quoted&quot;"),
            "Ben & Jerry's <tag> \"quoted\""
        );
    }

    #[test]
    fn nbsp_becomes_a_space() {
        assert_eq!(to_plain_text("a&nbsp;b"), "a b");
    }

    #[test]
    fn numeric_entities_are_decoded() {
        assert_eq!(
            to_plain_text("&#39;quoted&#39; &#x2014; dash"),
            "'quoted' — dash"
        );
    }

    #[test]
    fn bare_ampersand_is_kept() {
        assert_eq!(to_plain_text("Tom & Jerry"), "Tom & Jerry");
        assert_eq!(to_plain_text("a &notanentity; b"), "a &notanentity; b");
    }

    #[test]
    fn unterminated_tag_is_text() {
        assert_eq!(
            to_plain_text("2 < 3 and that is true"),
            "2 < 3 and that is true"
        );
    }

    #[test]
    fn empty_input_yields_empty_output() {
        assert_eq!(to_plain_text(""), "");
        assert_eq!(to_plain_text("<div></div>"), "");
    }

    #[test]
    fn unicode_is_preserved() {
        assert_eq!(
            to_plain_text("<div>Заметка 📝 café</div>"),
            "Заметка 📝 café"
        );
    }

    #[test]
    fn realistic_body_shrinks_by_two_thirds() {
        let html = concat!(
            r#"<div><h1>Shopping list</h1></div><div><br></div>"#,
            r#"<div><ul class="Apple-dash-list"><li>milk</li><li>eggs</li>"#,
            r#"<li>butter</li></ul></div><div><br></div>"#,
            r#"<div><span style="font-weight: bold">Before Friday</span></div>"#,
        );
        let text = to_plain_text(html);
        assert_eq!(text, "Shopping list\nmilk\neggs\nbutter\nBefore Friday");
        assert!(
            text.len() * 3 < html.len(),
            "{} chars of text from {} of HTML; expected at least a 3x cut",
            text.len(),
            html.len()
        );
    }
}
