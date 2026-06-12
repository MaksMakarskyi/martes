pub mod errors;
mod openers;

use super::block::*;
use super::document;
use openers::{OpenResult, try_open};
// use super::inline::*;
use errors::ParserError;
use std::collections::HashMap;
use std::vec;

pub fn parse<'a>(input: &'a str) -> Result<document::Document<'a>, ParserError> {
    let mut parser = Parser::new();
    parser.parse(input)?;

    Ok(parser.into())
}

struct Parser<'a> {
    doc: Vec<Block<'a>>,
    stack: Vec<Block<'a>>,
    links: HashMap<&'a str, LinkReference<'a>>,
}

impl<'a> Parser<'a> {
    fn new() -> Self {
        Parser {
            doc: Vec::new(),
            stack: Vec::new(),
            links: HashMap::new(),
        }
    }

    fn parse(&mut self, input: &'a str) -> Result<(), ParserError> {
        for line in input.lines() {
            self.parse_line(line)?;
        }

        self.close_from(0)?;

        // let mut stack = Vec::new();
        // for b in self.doc.iter_mut() {
        //     stack.push(b);
        // }

        // while let Some(b) = stack.pop() {
        //     match b {
        //         Block::ThematicBreak => continue,
        //         Block::LinkReference(_) => continue,
        //         Block::ATXHeading(ah) => process_inline(&mut ah.content)?,
        //         Block::Paragraph(ic) => process_inline(ic)?,
        //         Block::IndentedCode(ic) => process_inline(&mut ic.content)?,
        //         Block::FencedCode(fc) => process_inline(&mut fc.content)?,
        //         Block::BlockQuote(bc) => {
        //             for bc_b in bc.children.iter_mut() {
        //                 stack.push(bc_b);
        //             }
        //         }
        //     }
        // }

        Ok(())
    }

    fn parse_line(&mut self, line: &'a str) -> Result<(), ParserError> {
        let mut continuation = line;
        let mut close_from = 0;
        for block in self.stack.iter() {
            match self.try_continue(block, continuation) {
                ContinueResult::Continue(s) => {
                    continuation = s;
                    close_from += 1;
                }
                ContinueResult::Close => {
                    self.close_from(close_from)?;
                    return Ok(());
                }
                ContinueResult::NotContinue => break,
            }
        }

        // New block opening
        let mut last = if close_from > 0 {
            Some(&self.stack[close_from - 1])
        } else {
            None
        };

        let mut opened_blocks = Vec::new();
        loop {
            match try_open(continuation, last) {
                OpenResult::Continue(b, l) => {
                    opened_blocks.push(b);
                    continuation = l;
                }
                OpenResult::Opened(b) => {
                    self.close_from(close_from)?;
                    self.stack.append(&mut opened_blocks);
                    self.stack.push(b);
                    return Ok(());
                }
                OpenResult::NotOpened => break,
            }
            last = None;
        }

        if !opened_blocks.is_empty() {
            self.close_from(close_from)?;
            self.stack.append(&mut opened_blocks);
        }

        // Lazy continuation handling
        if !continuation.trim().is_empty()
            && let Some(Block::Paragraph(ic)) = self.stack.last_mut()
        {
            let InlineContent::Raw(lines) = ic else {
                return Err(ParserError::ExpectedRawContent);
            };
            lines.push(continuation);
            return Ok(());
        }

        self.close_from(close_from)?;

        // Remaning line handling
        if let Some(b) = self.stack.last_mut() {
            match b {
                Block::IndentedCode(ic) => {
                    let InlineContent::Raw(lines) = &mut ic.content else {
                        return Err(ParserError::ExpectedRawContent);
                    };
                    lines.push(continuation);
                }
                Block::FencedCode(fc) => {
                    let InlineContent::Raw(lines) = &mut fc.content else {
                        return Err(ParserError::ExpectedRawContent);
                    };
                    lines.push(continuation);
                }
                _ => {
                    if !continuation.trim().is_empty() {
                        self.stack
                            .push(Block::Paragraph(InlineContent::Raw(vec![continuation])));
                    }
                }
            }
        } else if !continuation.trim().is_empty() {
            self.stack
                .push(Block::Paragraph(InlineContent::Raw(vec![continuation])));
        }

        Ok(())
    }

    fn try_continue(&self, block: &Block, line: &'a str) -> ContinueResult<'a> {
        match block {
            Block::ThematicBreak | Block::ATXHeading(_) | Block::Paragraph(_) => {
                ContinueResult::NotContinue
            }
            Block::LinkReference(_) => unimplemented!(),
            Block::List(_) => unimplemented!(),
            Block::ListItem(_) => unimplemented!(),
            Block::IndentedCode(_) => self.try_continue_indented_code(line),
            Block::FencedCode(fc) => self.try_continue_fenced_code(fc, line),
            Block::BlockQuote(_) => self.try_continue_blockquote(line),
        }
    }

    fn try_continue_indented_code(&self, line: &'a str) -> ContinueResult<'a> {
        match line.strip_prefix("    ") {
            Some(s) => ContinueResult::Continue(s),
            None => ContinueResult::NotContinue,
        }
    }

    fn try_continue_fenced_code(&self, fc: &FencedCode, line: &'a str) -> ContinueResult<'a> {
        let after_indent = line.trim_start_matches(' ');
        let indent_size = line.len() - after_indent.len();
        if indent_size > 3 {
            return ContinueResult::Continue(line);
        }

        let after_tabs = after_indent.trim_start_matches('\t');
        if after_tabs.len() < after_indent.len() {
            return ContinueResult::Continue(line);
        }

        let fence_type: FenceType;
        let fence_occ: usize;
        let mut after_fence = after_tabs.trim_start_matches('~');
        if after_fence.len() == after_tabs.len() {
            after_fence = after_tabs.trim_start_matches('`');
            if after_fence.len() == after_tabs.len() {
                return ContinueResult::Continue(line);
            }

            fence_type = FenceType::Backtick;
        } else {
            fence_type = FenceType::Tilda;
        }

        fence_occ = after_tabs.len() - after_fence.len();

        // Closing sequence found
        if fence_type == fc.fence_type && fence_occ >= fc.fence_occ {
            return ContinueResult::Close;
        }

        ContinueResult::Continue(line)
    }

    fn try_continue_blockquote(&self, line: &'a str) -> ContinueResult<'a> {
        match line.strip_prefix(">") {
            Some(after_marker) => match after_marker.strip_prefix(" ") {
                Some(after_space) => ContinueResult::Continue(after_space),
                None => ContinueResult::Continue(after_marker),
            },
            None => ContinueResult::NotContinue,
        }
    }

    /// Closes open blocks on the stack starting from particular index inclusively
    fn close_from(&mut self, idx: usize) -> Result<(), ParserError> {
        if self.stack.len() == 0 || idx == self.stack.len() {
            return Ok(());
        }

        let num_iters = self.stack.len() - idx;
        for _ in 0..num_iters {
            let last = self.stack.pop().unwrap();

            if let Block::LinkReference(lr) = last {
                self.links.insert(lr.label, lr);
                continue;
            }

            let Some(prev_to_last) = self.stack.last_mut() else {
                // current last was the top most in the open blocks stack,
                // since there is no previous to the last
                self.doc.push(last);
                return Ok(());
            };

            match prev_to_last {
                Block::BlockQuote(children) => {
                    children.push(last);
                }
                _ => {
                    return Err(ParserError::InvalidContainer {
                        block_type: format!("{:?}", prev_to_last),
                    });
                }
            }
        }

        Ok(())
    }
}

impl<'a> Into<document::Document<'a>> for Parser<'a> {
    fn into(self) -> document::Document<'a> {
        document::Document::from(self.doc)
    }
}

#[derive(PartialEq, Debug, Clone)]
enum ContinueResult<'a> {
    Continue(&'a str),
    NotContinue,
    Close,
}

// fn process_inline(ic: &mut InlineContent) -> Result<(), ParserError> {
//     let InlineContent::Raw(lines) = ic else {
//         return Err(ParserError::ExpectedRawContent);
//     };

//     let mut parsed: Vec<Inline> = Vec::new();

//     let raw = lines.join("\n").into_bytes();
//     let mut idx = 0;
//     while idx < raw.len() {
//         match raw[idx] {
//             b'`' => match process_code_span(&mut raw[idx..]) {
//                 ProcessResult::Inline(cs, shift) => {
//                     idx += shift;
//                     parsed.push(cs);
//                 }
//                 ProcessResult::None => {
//                     while raw[idx] == b'`' {
//                         idx += 1
//                     }
//                 }
//             },
//             _ => idx += 1,
//         }
//     }

//     Ok(())
// }

// fn process_code_span<'a>(content: &'a mut [u8]) -> ProcessResult<'a> {
//     let mut idx = 0;
//     let mut num_markers = 0;

//     while content[idx] == b'`' {
//         idx += 1
//     }
//     num_markers = idx;

//     let mut num_current = 0;
//     while idx < content.len() {
//         match content[idx] {
//             b'`' => {
//                 num_current += 1;
//                 if idx == content.len() - 1 || content[idx + 1] != b'`' {
//                     break;
//                 }
//             }
//             _ => {
//                 num_current = 0;
//             }
//         }
//     }

//     if num_current != num_markers {
//         return ProcessResult::None;
//     }

//     for b in content.iter_mut() {
//         if *b == b'\n' {
//             *b = b' ';
//         }
//     }

//     if &content[0 + num_markers] == &b' ' && &content[idx - num_markers] == &b' ' {
//         return ProcessResult::Inline(
//             Inline::CodeSpan(
//                 str::from_utf8(&content[0 + num_markers + 1..idx - num_markers - 1]).unwrap(),
//             ),
//             idx + 1,
//         );
//     } else {
//         return ProcessResult::Inline(
//             Inline::CodeSpan(str::from_utf8(&content[0 + num_markers..idx - num_markers]).unwrap()),
//             idx + 1,
//         );
//     }
// }

// enum ProcessResult<'a> {
//     Inline(Inline<'a>, usize),
//     None,
// }
