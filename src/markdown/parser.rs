pub mod errors;

use std::vec;

use super::block::*;
use super::document;
use errors::ParserError;

pub fn parse<'a>(input: &'a str) -> Result<document::Document<'a>, ParserError> {
    let mut parser = Parser::new();
    parser.parse(input)?;

    Ok(parser.into())
}

struct Parser<'a> {
    doc: document::Document<'a>,
    stack: Vec<Block<'a>>,
}

impl<'a> Parser<'a> {
    fn new() -> Self {
        Parser {
            doc: document::Document::new(),
            stack: Vec::new(),
        }
    }

    fn parse(&mut self, input: &'a str) -> Result<(), ParserError> {
        for line in input.lines() {
            self.process_line(line)?;
        }

        self.close_from(0)?;
        Ok(())
    }

    fn process_line(&mut self, line: &'a str) -> Result<(), ParserError> {
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
        let last = if close_from > 0 {
            Some(&self.stack[close_from - 1])
        } else {
            None
        };

        // TODO: for container blocks: iterate like in the try_continue, and add new blocks to the stack,
        // not just to the children attribute, since they are then treated as closed ones, which is
        // incorrect
        if let Some(new_block) = self.try_open(continuation, last) {
            self.close_from(close_from)?;
            self.stack.push(new_block);
            return Ok(());
        }

        println!("{:?}", self.stack);

        // Lazy continuation handling
        if !line.trim().is_empty()
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
            Block::ThematicBreak => ContinueResult::NotContinue,
            Block::ATXHeading(_) => ContinueResult::NotContinue,
            Block::IndentedCode(_) => match line.strip_prefix("    ") {
                Some(s) => ContinueResult::Continue(s),
                None => ContinueResult::NotContinue,
            },
            Block::FencedCode(fc) => self.try_continue_fenced_code(fc, line),
            Block::Paragraph(_) => ContinueResult::NotContinue,
            Block::BlockQuote(_) => match line.strip_prefix(">") {
                Some(s) => ContinueResult::Continue(s),
                None => ContinueResult::NotContinue,
            },
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

    fn try_open(&self, line: &'a str, last: Option<&Block<'a>>) -> Option<Block<'a>> {
        if let Some(b) = self.try_open_idented_code(line, last) {
            return Some(b);
        }

        // TODO: check for the Setext Heading

        if let Some(b) = self.try_open_thematic_break(line) {
            return Some(b);
        }
        if let Some(b) = self.try_open_atx_heading(line) {
            return Some(b);
        }
        if let Some(b) = self.try_open_fenced_code(line, last) {
            return Some(b);
        }
        if let Some(b) = self.try_open_block_quote(line) {
            return Some(b);
        }

        // self.try_open_paragraph(line)
        None
    }

    fn try_open_idented_code(&self, line: &'a str, last: Option<&Block<'a>>) -> Option<Block<'a>> {
        if let Some(Block::Paragraph(_)) = last {
            return None;
        }
        if let Some(Block::FencedCode(_)) = last {
            return None;
        }
        if let Some(s) = line.strip_prefix("    ") {
            return Some(Block::IndentedCode(IndentedCode {
                content: InlineContent::Raw(vec![s]),
            }));
        }
        if let Some(s) = line.strip_prefix("\t") {
            return Some(Block::IndentedCode(IndentedCode {
                content: InlineContent::Raw(vec![s]),
            }));
        }

        None
    }

    fn try_open_thematic_break(&self, line: &'a str) -> Option<Block<'a>> {
        let after_space_indent = line.trim_start_matches(' ');
        if line.len() - after_space_indent.len() > 3 {
            return None;
        }

        let after_indent = after_space_indent.trim_start_matches('\t');
        if after_space_indent.len() > after_indent.len() {
            return None;
        }

        let mut thematic_ch = None;
        let mut occ = 0;
        for ch in after_indent.chars() {
            match ch {
                '-' | '_' | '*' => {
                    if let Some(thematic_ch) = thematic_ch {
                        if thematic_ch != ch {
                            return None;
                        } else {
                            occ += 1;
                        }
                    } else {
                        thematic_ch = Some(ch);
                        occ += 1;
                    }
                }
                '\t' | ' ' => continue,
                _ => return None,
            }
        }

        if occ < 3 {
            return None;
        }

        Some(Block::ThematicBreak)
    }

    fn try_open_atx_heading(&self, line: &'a str) -> Option<Block<'a>> {
        let after_indent = line.trim_start_matches(' ');
        if line.len() - after_indent.len() > 3 {
            return None;
        }

        let after_markers = after_indent.trim_start_matches('#');
        let level = after_indent.len() - after_markers.len();
        if level <= 0 || level > 6 {
            return None;
        }

        let Ok(level) = ATXHeadingLevel::try_from(level as u8) else {
            return None;
        };

        let after_whitespaces_before = after_markers.trim_start();
        if after_markers.len() == after_whitespaces_before.len() {
            if after_whitespaces_before.len() == 0 {
                return Some(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec![""]),
                    level,
                }));
            }

            return None;
        }

        let after_whitespaces = after_whitespaces_before.trim_end();
        let after_closing_seq = after_whitespaces.trim_end_matches('#');
        if after_closing_seq.len() == after_whitespaces.len() {
            return Some(Block::ATXHeading(ATXHeading {
                content: InlineContent::Raw(vec![after_closing_seq]),
                level,
            }));
        }

        let after_whitespaces_after = after_closing_seq.trim_end();
        if after_whitespaces_after.len() == after_closing_seq.len() {
            return Some(Block::ATXHeading(ATXHeading {
                content: InlineContent::Raw(vec![after_whitespaces]),
                level,
            }));
        }

        Some(Block::ATXHeading(ATXHeading {
            content: InlineContent::Raw(vec![after_whitespaces_after]),
            level,
        }))
    }

    fn try_open_fenced_code(&self, line: &'a str, last: Option<&Block<'a>>) -> Option<Block<'a>> {
        let after_indent = line.trim_start_matches(' ');
        let indent_size = line.len() - after_indent.len();
        if indent_size > 3 {
            return None;
        }

        let after_tabs = after_indent.trim_start_matches('\t');
        if after_tabs.len() < after_indent.len() {
            return None;
        }

        let fence_type: FenceType;
        let fence_occ: usize;
        let mut after_fence = after_tabs.trim_start_matches('~');
        if after_fence.len() == after_tabs.len() {
            after_fence = after_tabs.trim_start_matches('`');
            if after_fence.len() == after_tabs.len() {
                return None;
            }

            fence_type = FenceType::Backtick;
        } else {
            fence_type = FenceType::Tilda;
        }

        fence_occ = after_tabs.len() - after_fence.len();
        if fence_occ <= 2 {
            return None;
        }

        if let Some(Block::FencedCode(fc)) = last {
            if fc.fence_occ > fence_occ || fc.fence_type != fence_type {
                return None;
            }
        }

        let language = after_fence.trim_start().split(' ').next().unwrap_or("");

        Some(Block::FencedCode(FencedCode {
            content: InlineContent::Raw(Vec::new()),
            language: language,
            ident: indent_size,
            fence_type: fence_type,
            fence_occ: fence_occ,
        }))
    }

    fn try_open_block_quote(&self, line: &'a str) -> Option<Block<'a>> {
        let after_indent = line.trim_start_matches(' ');
        let indent_size = line.len() - after_indent.len();
        if indent_size > 3 {
            return None;
        }

        let after_tabs = after_indent.trim_start_matches('\t');
        if after_tabs.len() < after_indent.len() {
            return None;
        }

        let after_marker = after_tabs.strip_prefix('>')?;
        let after_whitespaces_before = after_marker.trim_start();

        let mut children = Vec::new();
        match self.try_open(after_whitespaces_before, None) {
            Some(block) => children.push(block),
            None => {
                if !after_whitespaces_before.trim().is_empty() {
                    children.push(Block::Paragraph(InlineContent::Raw(vec![
                        after_whitespaces_before,
                    ])))
                }
            }
        }

        Some(Block::BlockQuote(BlockQuote { children: children }))
    }

    /// Closes open blocks on the stack starting from particular index inclusively
    fn close_from(&mut self, idx: usize) -> Result<(), ParserError> {
        if self.stack.len() == 0 || idx == self.stack.len() {
            return Ok(());
        }

        let num_iters = self.stack.len() - idx;
        for _ in 0..num_iters {
            let last = self.stack.pop().unwrap();

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

enum ContinueResult<'a> {
    Continue(&'a str),
    NotContinue,
    Close,
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Case<'a> {
        name: &'static str,
        input: &'static str,
        expected: Option<Block<'a>>,
    }

    #[test]
    fn test_try_open_indented_code() {
        let tests = vec![
            Case {
                name: "opens_at_with_four_space_prefix",
                input: "    let abc = 'some var'",
                expected: Some(Block::IndentedCode(IndentedCode {
                    content: InlineContent::Raw(vec!["let abc = 'some var'"]),
                })),
            },
            Case {
                name: "opens_at_with_tab_prefix",
                input: "\ta = np.array()",
                expected: Some(Block::IndentedCode(IndentedCode {
                    content: InlineContent::Raw(vec!["a = np.array()"]),
                })),
            },
            Case {
                name: "leaves_spaces_after_four_space_prefix",
                input: "      a = np.array()",
                expected: Some(Block::IndentedCode(IndentedCode {
                    content: InlineContent::Raw(vec!["  a = np.array()"]),
                })),
            },
            Case {
                name: "leaves_spaces_after_tab",
                input: "\t  a = np.array()",
                expected: Some(Block::IndentedCode(IndentedCode {
                    content: InlineContent::Raw(vec!["  a = np.array()"]),
                })),
            },
            Case {
                name: "insufficient_spaces",
                input: "   fff",
                expected: None,
            },
            Case {
                name: "no_spaces",
                input: "dfsf",
                expected: None,
            },
        ];

        let parser = Parser::new();
        for test in tests {
            let output = parser.try_open_idented_code(test.input, None);
            assert_eq!(output, test.expected, "case: {}", test.name);
        }
    }

    #[test]
    fn test_try_open_thematic_break() {
        let tests = vec![
            Case {
                name: "few_markers",
                input: "--",
                expected: None,
            },
            Case {
                name: "several_marker_types",
                input: "-*-",
                expected: None,
            },
            Case {
                name: "invalid_marker",
                input: "+++",
                expected: None,
            },
            Case {
                name: "does_not_interrupt_indented_code",
                input: "    ---",
                expected: None,
            },
            Case {
                name: "does_not_interrupt_tab_prefixed_indented_code",
                input: "\t---",
                expected: None,
            },
            Case {
                name: "tab_and_spaces_ident",
                input: " \t---",
                expected: None,
            },
            Case {
                name: "invalid_characters_after",
                input: "--- ds",
                expected: None,
            },
            Case {
                name: "invalid_characters_before",
                input: "ewr---",
                expected: None,
            },
            Case {
                name: "invalid_characters_in_between",
                input: "-f-f-",
                expected: None,
            },
            Case {
                name: "hyphen_markers",
                input: "---",
                expected: Some(Block::ThematicBreak),
            },
            Case {
                name: "asterisk_markers",
                input: "***",
                expected: Some(Block::ThematicBreak),
            },
            Case {
                name: "underline_markers",
                input: "___",
                expected: Some(Block::ThematicBreak),
            },
            Case {
                name: "more_than_three_marker_occurrences",
                input: "_____________________________________",
                expected: Some(Block::ThematicBreak),
            },
            Case {
                name: "spaces_and_tabs_in_between",
                input: " **  * **\t* ** * **\t   ",
                expected: Some(Block::ThematicBreak),
            },
        ];

        let parser = Parser::new();
        for test in tests {
            let output = parser.try_open_thematic_break(test.input);
            assert_eq!(output, test.expected, "case: {}", test.name);
        }
    }

    #[test]
    fn test_try_open_atx_heading() {
        let tests = vec![
            Case {
                name: "escape_marker_before",
                input: "\\### foo",
                expected: None,
            },
            Case {
                name: "escape_marker_inside",
                input: "##\\# foo",
                expected: None,
            },
            Case {
                name: "too_many_markers",
                input: "######### foo",
                expected: None,
            },
            Case {
                name: "too_many_spaces_before_markers",
                input: "    # foo",
                expected: None,
            },
            Case {
                name: "tab_before_markers",
                input: "\t# foo",
                expected: None,
            },
            Case {
                name: "spaces_and_tab_before_markers",
                input: "  \t# foo",
                expected: None,
            },
            Case {
                name: "h1_heading",
                input: "# foo",
                expected: Some(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo"]),
                    level: ATXHeadingLevel::H1,
                })),
            },
            Case {
                name: "h2_heading",
                input: "## foo",
                expected: Some(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo"]),
                    level: ATXHeadingLevel::H2,
                })),
            },
            Case {
                name: "h3_heading",
                input: "### foo",
                expected: Some(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo"]),
                    level: ATXHeadingLevel::H3,
                })),
            },
            Case {
                name: "h4_heading",
                input: "#### foo",
                expected: Some(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo"]),
                    level: ATXHeadingLevel::H4,
                })),
            },
            Case {
                name: "h5_heading",
                input: "##### foo",
                expected: Some(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo"]),
                    level: ATXHeadingLevel::H5,
                })),
            },
            Case {
                name: "h6_heading",
                input: "###### foo",
                expected: Some(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo"]),
                    level: ATXHeadingLevel::H6,
                })),
            },
            Case {
                name: "empty_content",
                input: "#",
                expected: Some(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec![""]),
                    level: ATXHeadingLevel::H1,
                })),
            },
            Case {
                name: "space_content",
                input: "## ",
                expected: Some(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec![""]),
                    level: ATXHeadingLevel::H2,
                })),
            },
            Case {
                name: "tab_content",
                input: "###\t",
                expected: Some(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec![""]),
                    level: ATXHeadingLevel::H3,
                })),
            },
            Case {
                name: "spaces_around",
                input: "######                  foo                     ",
                expected: Some(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo"]),
                    level: ATXHeadingLevel::H6,
                })),
            },
            Case {
                name: "closing_sequence",
                input: "## foo ##",
                expected: Some(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo"]),
                    level: ATXHeadingLevel::H2,
                })),
            },
            Case {
                name: "not_a_closing_sequence",
                input: "## foo ## b",
                expected: Some(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo ## b"]),
                    level: ATXHeadingLevel::H2,
                })),
            },
            Case {
                name: "escaped_closing_sequence_inside",
                input: "## foo #\\##",
                expected: Some(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo #\\##"]),
                    level: ATXHeadingLevel::H2,
                })),
            },
            Case {
                name: "escaped_closing_sequence_before",
                input: "## foo \\###",
                expected: Some(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo \\###"]),
                    level: ATXHeadingLevel::H2,
                })),
            },
            Case {
                name: "longer_closing_sequence",
                input: "# foo ##################################",
                expected: Some(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo"]),
                    level: ATXHeadingLevel::H1,
                })),
            },
            Case {
                name: "shorter_closing_sequence",
                input: "##### foo ##",
                expected: Some(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo"]),
                    level: ATXHeadingLevel::H5,
                })),
            },
            Case {
                name: "spaces_after_closing_sequence",
                input: "### foo ###     ",
                expected: Some(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo"]),
                    level: ATXHeadingLevel::H3,
                })),
            },
            Case {
                name: "tabs_after_closing_sequence",
                input: "### foo ###     ",
                expected: Some(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo"]),
                    level: ATXHeadingLevel::H3,
                })),
            },
            Case {
                name: "closing_sequence_without_a_space",
                input: "### foo#",
                expected: Some(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo#"]),
                    level: ATXHeadingLevel::H3,
                })),
            },
            Case {
                name: "closing_sequence_after_tab",
                input: "### foo\t#",
                expected: Some(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo"]),
                    level: ATXHeadingLevel::H3,
                })),
            },
            Case {
                name: "three_spaces_before_markers",
                input: "   # foo",
                expected: Some(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo"]),
                    level: ATXHeadingLevel::H1,
                })),
            },
        ];

        let parser = Parser::new();
        for test in tests {
            let output = parser.try_open_atx_heading(test.input);
            assert_eq!(output, test.expected, "case: {}", test.name);
        }
    }

    #[test]
    fn test_try_open_fenced_code() {
        let tests = vec![
            Case {
                name: "too_many_spaces_before_markers",
                input: "    ~~~",
                expected: None,
            },
            Case {
                name: "tab_before_markers",
                input: "\t```",
                expected: None,
            },
            Case {
                name: "tab_and_spaces_before_markers",
                input: " \t ~~~",
                expected: None,
            },
            Case {
                name: "too_few_markers",
                input: "``",
                expected: None,
            },
            Case {
                name: "tildes",
                input: "~~~",
                expected: Some(Block::FencedCode(FencedCode {
                    content: InlineContent::Raw(Vec::new()),
                    language: "",
                    ident: 0,
                    fence_type: FenceType::Tilda,
                    fence_occ: 3,
                })),
            },
            Case {
                name: "Backticks",
                input: "```",
                expected: Some(Block::FencedCode(FencedCode {
                    content: InlineContent::Raw(Vec::new()),
                    language: "",
                    ident: 0,
                    fence_type: FenceType::Backtick,
                    fence_occ: 3,
                })),
            },
            Case {
                name: "spaces_before_markers",
                input: "   ```",
                expected: Some(Block::FencedCode(FencedCode {
                    content: InlineContent::Raw(Vec::new()),
                    language: "",
                    ident: 3,
                    fence_type: FenceType::Backtick,
                    fence_occ: 3,
                })),
            },
            Case {
                name: "language",
                input: " ~~~python",
                expected: Some(Block::FencedCode(FencedCode {
                    content: InlineContent::Raw(Vec::new()),
                    language: "python",
                    ident: 1,
                    fence_type: FenceType::Tilda,
                    fence_occ: 3,
                })),
            },
            Case {
                name: "language_with_space",
                input: " ~~~ rust",
                expected: Some(Block::FencedCode(FencedCode {
                    content: InlineContent::Raw(Vec::new()),
                    language: "rust",
                    ident: 1,
                    fence_type: FenceType::Tilda,
                    fence_occ: 3,
                })),
            },
            Case {
                name: "language_with_few_spaces",
                input: " ~~~     rust",
                expected: Some(Block::FencedCode(FencedCode {
                    content: InlineContent::Raw(Vec::new()),
                    language: "rust",
                    ident: 1,
                    fence_type: FenceType::Tilda,
                    fence_occ: 3,
                })),
            },
            Case {
                name: "empty_language",
                input: " ~~~~      ",
                expected: Some(Block::FencedCode(FencedCode {
                    content: InlineContent::Raw(Vec::new()),
                    language: "",
                    ident: 1,
                    fence_type: FenceType::Tilda,
                    fence_occ: 4,
                })),
            },
            Case {
                name: "language_with_noize",
                input: " ~~~~ rust startline=3 $%@#$",
                expected: Some(Block::FencedCode(FencedCode {
                    content: InlineContent::Raw(Vec::new()),
                    language: "rust",
                    ident: 1,
                    fence_type: FenceType::Tilda,
                    fence_occ: 4,
                })),
            },
        ];

        let parser = Parser::new();
        for test in tests {
            let output = parser.try_open_fenced_code(test.input, None);
            assert_eq!(output, test.expected, "case: {}", test.name);
        }
    }

    #[test]
    fn test_try_open_block_quote() {
        let tests = vec![Case {
            name: "simple_quote",
            input: "> some text",
            expected: Some(Block::BlockQuote(BlockQuote {
                children: vec![Block::Paragraph(InlineContent::Raw(vec!["some text"]))],
            })),
        }];

        let parser = Parser::new();
        for test in tests {
            let output = parser.try_open_block_quote(test.input);
            assert_eq!(output, test.expected, "case: {}", test.name);
        }
    }
}
