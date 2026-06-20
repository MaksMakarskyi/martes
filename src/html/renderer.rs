use crate::markdown::{
    block::*,
    inline::{EmphasisType, Inline},
};

pub fn render_markdown<'a>(doc: Vec<Block<'a>>) -> String {
    let mut res = String::new();
    for b in doc.iter() {
        res.push_str(&render_markdown_block(b));
    }

    res
}

fn render_markdown_block(block: &Block) -> String {
    match block {
        Block::ThematicBreak => String::from("<hr />"),
        Block::ATXHeading(b) => render_markdown_atx(b),
        Block::IndentedCode(ic) => render_markdown_indented_code(ic),
        Block::FencedCode(fc) => render_markdown_fenced_code(fc),
        Block::Paragraph(ic) => render_markdown_paragraph(ic),
        // Block::LinkReference(_) => String::new(),
        Block::BlockQuote(bq) => render_markdown_block_quote(bq),
        Block::List(_) => String::new(),
        Block::ListItem(_) => String::new(),
    }
}

fn render_markdown_atx<'a>(b: &ATXHeading<'a>) -> String {
    let InlineContent::Parsed(inlines) = &b.content else {
        unreachable!("the block must be raw at this point");
    };

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
        inlines
            .iter()
            .map(render_markdown_inline)
            .collect::<Vec<_>>()
            .join("\n")
    )
}

fn render_markdown_fenced_code(fenced_code: &FencedCode) -> String {
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

fn render_markdown_indented_code(indented_code: &IndentedCode) -> String {
    let InlineContent::Parsed(inlines) = &indented_code.content else {
        unreachable!("the block must be raw at this point");
    };

    format!(
        "<pre><code>{}</code></pre>\n",
        inlines.iter().fold(String::new(), |mut acc, line| {
            acc.push_str(&render_markdown_inline(line));
            acc.push('\n');
            acc
        })
    )
}

fn render_markdown_paragraph(ic: &InlineContent) -> String {
    let InlineContent::Parsed(inlines) = ic else {
        unreachable!("the block must be raw at this point");
    };

    format!(
        "<p>{}</p>\n",
        inlines
            .iter()
            .map(render_markdown_inline)
            .collect::<Vec<_>>()
            .join("\n")
    )
}

fn render_markdown_block_quote(bq: &BlockQuote) -> String {
    let mut res = String::new();
    for child in bq.children.iter() {
        res.push_str(&render_markdown_block(child));
    }

    format!("<blockquote>\n{res}</blockquote>\n")
}

fn render_markdown_inline(inline_block: &Inline) -> String {
    match inline_block {
        Inline::HardLineBreak => String::from("<br />"),
        Inline::Autolink(link) => format!("<a href=\"{link}\">{link}</a>"),
        Inline::CodeSpan(span) => format!("<code>{span}</code>"),
        Inline::Emphasis(emphasis) => match emphasis.emphasis_type {
            EmphasisType::Common => format!("<em>{}</em>", emphasis.text),
            EmphasisType::Strong => format!("<strong>{}</strong>", emphasis.text),
        },
        Inline::HTML(html) => String::from(*html),
        Inline::Image(image) => format!(
            "<img src=\"{}\" alt=\"{}\"{} />",
            image.src,
            image.alt,
            render_title(image.title)
        ),
        Inline::Link(link) => format!(
            "<a href=\"{}\"{}>{}</a>",
            link.url,
            render_title(link.title),
            link.text,
        ),
        Inline::TextualContent(text) => String::from(*text),
    }
}

fn render_title<'a>(title: Option<&'a str>) -> String {
    match title {
        Some(text) => format!(" title=\"{text}\""),
        None => String::new(),
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
