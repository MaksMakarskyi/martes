use crate::markdown::block::{ATXHeading, ATXHeadingLevel, Block, InlineContent};

use super::block;

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
                Block::FencedCode(_) => html.push_str(""),
                Block::Paragraph(_) => html.push_str(""),
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

        let text = match b.clone().content {
            InlineContent::Raw(c) => c.join(" "),
            InlineContent::Parsed(_) => String::from("Inline text"),
        };

        format!("<{tag_name}>{}</{tag_name}>\n", text)
    }
}

impl<'a> From<Vec<block::Block<'a>>> for Document<'a> {
    fn from(value: Vec<block::Block<'a>>) -> Self {
        Self { children: value }
    }
}
