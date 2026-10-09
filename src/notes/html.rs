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

pub fn to_plain_text(html: &str) -> String {
    let mut out = PlainText::with_capacity(html.len() / 2);
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
            out.line_break();
        }
        rest = &after_open[close + 1..];
    }

    out.finish()
}

/// Plain text tidied as it is written: each line loses its trailing
/// whitespace when it ends, runs of blank lines collapse to one, and the ends
/// are trimmed. Tidying on the fly keeps a body's conversion to one buffer.
struct PlainText {
    out: String,
    /// Where the line being written starts in `out`.
    line_start: usize,
    blank_run: usize,
    /// Whether the text written so far, before tidying, ends partway through
    /// a line. Line-breaking tags only end a line that has started.
    in_line: bool,
}

impl PlainText {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            out: String::with_capacity(capacity),
            line_start: 0,
            blank_run: 0,
            in_line: false,
        }
    }

    fn push_str(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        let mut lines = text.split('\n');
        if let Some(first) = lines.next() {
            self.out.push_str(first);
        }
        for line in lines {
            self.end_line();
            self.out.push_str(line);
        }
        self.in_line = !text.ends_with('\n');
    }

    fn push(&mut self, ch: char) {
        self.push_str(ch.encode_utf8(&mut [0; 4]));
    }

    fn line_break(&mut self) {
        if self.in_line {
            self.end_line();
            self.in_line = false;
        }
    }

    fn end_line(&mut self) {
        let line_len = self.trim_line();
        if line_len == 0 {
            self.blank_run += 1;
            if self.blank_run > 1 || self.line_start == 0 {
                return;
            }
        } else {
            self.blank_run = 0;
        }
        self.out.push('\n');
        self.line_start = self.out.len();
    }

    /// Drop the current line's trailing whitespace and return its length.
    fn trim_line(&mut self) -> usize {
        let line_len = self.out[self.line_start..].trim_end().len();
        self.out.truncate(self.line_start + line_len);
        line_len
    }

    fn finish(mut self) -> String {
        self.trim_line();
        let len = self.out.trim_end_matches('\n').len();
        self.out.truncate(len);
        self.out
    }
}

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

/// Where decoded text goes: [`PlainText`] in production, a bare `String` for
/// the reference implementation in the tests.
trait TextSink {
    fn push_str(&mut self, text: &str);
    fn push(&mut self, ch: char);
}

impl TextSink for PlainText {
    fn push_str(&mut self, text: &str) {
        PlainText::push_str(self, text);
    }

    fn push(&mut self, ch: char) {
        PlainText::push(self, ch);
    }
}

#[cfg(test)]
impl TextSink for String {
    fn push_str(&mut self, text: &str) {
        String::push_str(self, text);
    }

    fn push(&mut self, ch: char) {
        String::push(self, ch);
    }
}

fn push_decoded(out: &mut impl TextSink, text: &str) {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Reference implementation: build the raw text, then tidy it into a second
    /// buffer.
    fn two_pass(html: &str) -> String {
        let mut raw = String::new();
        let mut rest = html;
        loop {
            let Some(open) = rest.find('<') else {
                push_decoded(&mut raw, rest);
                break;
            };
            push_decoded(&mut raw, &rest[..open]);
            let after_open = &rest[open + 1..];
            let Some(close) = after_open.find('>') else {
                push_decoded(&mut raw, &rest[open..]);
                break;
            };
            if breaks_line(&after_open[..close]) && !raw.is_empty() && !raw.ends_with('\n') {
                raw.push('\n');
            }
            rest = &after_open[close + 1..];
        }

        let mut out = String::new();
        let mut blank_run = 0;
        for line in raw.lines() {
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

    #[test]
    fn single_pass_matches_two_pass_on_every_short_combination() {
        const FRAGMENTS: &[&str] = &[
            "a", " ", "\n", "\r\n", "<div>", "</div>", "<br>", "<b>", "&nbsp;", "&#10;", "  b  ",
            "\t",
        ];
        let mut html = String::new();
        let mut indices = [0usize; 4];
        for len in 1..=indices.len() {
            loop {
                html.clear();
                for &i in &indices[..len] {
                    html.push_str(FRAGMENTS[i]);
                }
                assert_eq!(to_plain_text(&html), two_pass(&html), "input: {html:?}");

                let Some(pos) = indices[..len]
                    .iter()
                    .rposition(|&i| i + 1 < FRAGMENTS.len())
                else {
                    break;
                };
                indices[pos] += 1;
                indices[pos + 1..len].fill(0);
            }
            indices = [0; 4];
        }
    }

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
