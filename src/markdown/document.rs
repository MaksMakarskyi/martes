use super::block;
use crate::markdown::block::{ATXHeading, ATXHeadingLevel, Block, FencedCode, InlineContent};

#[derive(Debug, PartialEq, Clone)]
pub struct Document<'a> {
    children: Vec<block::Block<'a>>,
}

impl<'a> Document<'a> {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }

    pub fn push(&mut self, b: Block<'a>) {
        self.children.push(b);
    }

    pub fn to_html(&self) -> String {
        let mut html = String::new();

        for b in self.children.iter() {
            match b {
                Block::ThematicBreak => html.push_str("<hr />\n"),
                Block::ATXHeading(b) => {
                    let s = self.atx_to_html(b);
                    html.push_str(&s);
                }
                Block::IndentedCode(_) => html.push_str(""),
                Block::FencedCode(fc) => html.push_str(&self.fenced_code_to_html(fc)),
                Block::Paragraph(ic) => {
                    let s = format!("<p>{}</p>\n", self.inline_content_to_html(ic));
                    html.push_str(&s);
                }
                Block::BlockQuote(_) => html.push_str(""),
            }
        }

        html
    }

    fn atx_to_html(&self, b: &ATXHeading<'a>) -> String {
        let tag_name = match b.level {
            ATXHeadingLevel::H1 => "h1",
            ATXHeadingLevel::H2 => "h2",
            ATXHeadingLevel::H3 => "h3",
            ATXHeadingLevel::H4 => "h4",
            ATXHeadingLevel::H5 => "h5",
            ATXHeadingLevel::H6 => "h6",
        };

        format!(
            "<{tag_name}>{}</{tag_name}>\n",
            self.inline_content_to_html(&b.content)
        )
    }

    fn inline_content_to_html(&self, inline_content: &InlineContent) -> String {
        match inline_content {
            InlineContent::Raw(c) => String::from(&c.join(" ")),
            InlineContent::Parsed(_) => String::from("Inline text"),
        }
    }

    fn fenced_code_to_html(&self, fenced_code: &FencedCode) -> String {
        let language = match fenced_code.language {
            "" => "",
            _ => &format!(" class=\"language-{}\"", fenced_code.language),
        };

        let content = match &fenced_code.content {
            InlineContent::Raw(lines) => lines
                .iter()
                .map(|&line| strip_max_leading_spaces(line, fenced_code.ident))
                .fold(String::new(), |mut acc, line| {
                    acc.push_str(line);
                    acc.push('\n');
                    acc
                }),
            InlineContent::Parsed(_) => String::from("Inline text"),
        };

        format!("<pre><code{language}>{content}</code></pre>\n")
    }
}

impl<'a> From<Vec<block::Block<'a>>> for Document<'a> {
    fn from(value: Vec<block::Block<'a>>) -> Self {
        Self { children: value }
    }
}

fn strip_max_leading_spaces(mut s: &str, max: usize) -> &str {
    for _ in 0..max {
        match s.strip_prefix(' ') {
            Some(rest) => s = rest,
            None => break,
        }
    }
    s
}
