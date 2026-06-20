use crate::markdown::block::*;
use crate::markdown::inline::*;

pub fn parse_inline(doc: &mut [Block]) {
    let mut stack = Vec::new();
    for b in doc.iter_mut() {
        stack.push(b);
    }

    while let Some(b) = stack.pop() {
        match b {
            Block::ThematicBreak => continue,
            // Block::LinkReference(_) => continue,
            Block::ATXHeading(ah) => parse_inline_content(&mut ah.content),
            Block::Paragraph(ic) => parse_inline_content(ic),
            Block::IndentedCode(ic) => parse_inline_content(&mut ic.content),
            Block::FencedCode(fc) => parse_inline_content(&mut fc.content),
            Block::BlockQuote(bc) => stack.extend(bc.children.iter_mut()),
            Block::List(list) => stack.extend(list.items.iter_mut()),
            Block::ListItem(li) => stack.extend(li.children.iter_mut()),
        }
    }
}

fn parse_inline_content(ic: &mut InlineContent) {
    let InlineContent::Raw(lines) = ic else {
        return;
    };

    let mut processed = Vec::new();
    for &mut line in lines {
        processed.push(Inline::TextualContent(line));
    }

    *ic = InlineContent::Parsed(processed);
}
