use super::block;
use crate::markdown::block::{
    ATXHeading, ATXHeadingLevel, Block, BlockQuote, FencedCode, IndentedCode, InlineContent,
};

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
            html.push_str(&self.block_to_html(b));
        }

        html
    }

    fn block_to_html(&self, block: &Block) -> String {
        match block {
            Block::ThematicBreak => String::from("<hr />"),
            Block::ATXHeading(b) => self.atx_to_html(b),
            Block::IndentedCode(ic) => self.indented_code_to_html(ic),
            Block::FencedCode(fc) => self.fenced_code_to_html(fc),
            Block::Paragraph(ic) => self.paragraph_to_html(ic),
            // Block::LinkReference(_) => String::new(),
            Block::BlockQuote(bq) => self.block_quote_html(bq),
            Block::List(_) => String::new(),
            Block::ListItem(_) => String::new(),
        }
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

    fn indented_code_to_html(&self, indented_code: &IndentedCode) -> String {
        let InlineContent::Raw(lines) = &indented_code.content else {
            unreachable!("the block must be raw at this point");
        };

        format!(
            "<pre><code>{}</code></pre>\n",
            lines.iter().fold(String::new(), |mut acc, line| {
                acc.push_str(line);
                acc.push('\n');
                acc
            })
        )
    }

    fn paragraph_to_html(&self, ic: &InlineContent) -> String {
        let InlineContent::Raw(lines) = ic else {
            unreachable!("the block must be raw at this point");
        };

        format!(
            "<p>{}</p>\n",
            lines
                .iter()
                .map(|l| {
                    let after_whitespaces_before = l.trim_start();
                    let after_whitespaces = after_whitespaces_before.trim_end();

                    let hard_line_break =
                        if after_whitespaces_before.len() - after_whitespaces.len() > 2 {
                            "<br />"
                        } else {
                            ""
                        };

                    return format!("{after_whitespaces}{hard_line_break}");
                })
                .collect::<Vec<_>>()
                .join("\n")
        )
    }

    fn block_quote_html(&self, bq: &BlockQuote) -> String {
        let mut res = String::new();
        for child in bq.children.iter() {
            res.push_str(&self.block_to_html(child));
        }

        format!("<blockquote>\n{res}</blockquote>\n")
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
