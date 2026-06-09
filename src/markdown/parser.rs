pub mod errors;

use super::block::*;
use super::document;

pub fn parse<'a>(input: &'a str) -> Result<document::Document<'a>, errors::ParserError> {
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

    fn parse(&mut self, input: &'a str) -> Result<(), errors::ParserError> {
        for line in input.lines() {
            self.process_line(line);
        }

        self.close_from(0);
        Ok(())
    }

    fn process_line(&mut self, line: &'a str) {
        let mut continuation = line;
        let mut close_from = 0;
        for block in self.stack.iter() {
            if let Some(s) = self.try_continue(block, continuation) {
                continuation = s;
                close_from += 1;
                continue;
            }

            // fenced code cannot be continued only in the case it is being closed,
            // so we close it, and skip the current line immediately, since it contains
            // only the closing fence (e.g. "~~~" or "```")
            if let Block::FencedCode(_) = block {
                self.close_from(close_from);
                return;
            }

            break;
        }

        if let Some(new_block) = self.try_open(continuation) {
            self.close_from(close_from);
            self.stack.push(new_block.clone());
            return;
        }

        // Remaning empty lines handling
        if line.trim().is_empty() {
            if let Some(Block::FencedCode(fc)) = self.stack.last_mut() {
                let InlineContent::Raw(lines) = &mut fc.content else {
                    unreachable!("the content must be raw at this point");
                };
                lines.push(continuation);
            }

            if let Some(Block::Paragraph(_)) = self.stack.last() {
                self.close_from(self.stack.len() - 1);
            }

            return;
        }

        self.close_from(close_from);

        // Remaning non-empty lines handling
        if let Some(b) = self.stack.last_mut() {
            match b {
                Block::IndentedCode(ic) => {
                    let InlineContent::Raw(lines) = &mut ic.content else {
                        unreachable!("the content must be raw at this point");
                    };
                    lines.push(continuation);
                }
                Block::FencedCode(fc) => {
                    let InlineContent::Raw(lines) = &mut fc.content else {
                        unreachable!("the content must be raw at this point");
                    };
                    lines.push(continuation);
                }
                Block::Paragraph(ic) => {
                    let InlineContent::Raw(lines) = ic else {
                        unreachable!("the content must be raw at this point");
                    };
                    lines.push(continuation);
                }
                _ => {
                    self.stack
                        .push(Block::Paragraph(InlineContent::Raw(vec![continuation])));
                }
            }
        } else {
            self.stack
                .push(Block::Paragraph(InlineContent::Raw(vec![continuation])));
        }
    }

    fn try_continue(&self, block: &Block, line: &'a str) -> Option<&'a str> {
        match block {
            Block::ThematicBreak => None,
            Block::ATXHeading(_) => None,
            Block::IndentedCode(_) => line.strip_prefix("    "),
            Block::FencedCode(_) => self.try_continue_fenced_code(block, line),
            Block::Paragraph(_) => match self.try_open(line) {
                Some(Block::ATXHeading(_))
                | Some(Block::FencedCode(_))
                | Some(Block::ThematicBreak) => None,
                _ => Some(line),
            },
            Block::BlockQuote(_) => line.strip_prefix("> "),
        }
    }

    fn try_continue_fenced_code(&self, block: &Block, line: &'a str) -> Option<&'a str> {
        let Block::FencedCode(fc) = block else {
            return Some(line);
        };

        let after_indent = line.trim_start_matches(' ');
        let indent_size = line.len() - after_indent.len();
        if indent_size > 3 {
            return Some(line);
        }

        let after_tabs = after_indent.trim_start_matches('\t');
        if after_tabs.len() < after_indent.len() {
            return Some(line);
        }

        let fence_type: FenceType;
        let fence_occ: usize;
        let mut after_fence = after_tabs.trim_start_matches('~');
        if after_fence.len() == after_tabs.len() {
            after_fence = after_tabs.trim_start_matches('`');
            if after_fence.len() == after_tabs.len() {
                return Some(line);
            }

            fence_type = FenceType::Backtick;
        } else {
            fence_type = FenceType::Tilda;
        }

        fence_occ = after_tabs.len() - after_fence.len();

        // Closing sequence found
        if fence_type == fc.fence_type && fence_occ >= fc.fence_occ {
            return None;
        }

        Some(line)
    }

    fn try_open(&self, line: &'a str) -> Option<Block<'a>> {
        if let Some(b) = self.try_open_idented_code(line) {
            return Some(b);
        }

        // TODO: check for the Setext Heading

        if let Some(b) = self.try_open_thematic_break(line) {
            return Some(b);
        }
        if let Some(b) = self.try_open_atx_heading(line) {
            return Some(b);
        }
        if let Some(b) = self.try_open_fenced_code(line) {
            return Some(b);
        }

        // self.try_open_paragraph(line)
        None
    }

    fn try_open_idented_code(&self, line: &'a str) -> Option<Block<'a>> {
        if let Some(Block::Paragraph(_)) = self.stack.last() {
            return None;
        }
        if let Some(Block::FencedCode(_)) = self.stack.last() {
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

    fn try_open_fenced_code(&self, line: &'a str) -> Option<Block<'a>> {
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

        if let Some(Block::FencedCode(fc)) = self.stack.last() {
            if fc.fence_occ > fence_occ || fc.fence_type != fence_type {
                return None;
            }
            unreachable!(
                "the closing fence with enough fence markers must be detected by the try_continue block"
            );
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

    // fn try_open_paragraph(&self, line: &'a str) -> Option<Block<'a>> {
    //     if let Some(Block::FencedCode(_)) = self.stack.last() {
    //         return None;
    //     }
    //     if let Some(Block::IndentedCode(_)) = self.stack.last() {
    //         return None;
    //     }

    //     if line.trim().is_empty() {
    //         return None;
    //     }

    //     return Some(Block::Paragraph(InlineContent::Raw(vec![line])));
    // }

    /// Closes open blocks on the stack starting from particular index inclusively
    fn close_from(&mut self, idx: usize) {
        if self.stack.len() == 0 || idx == self.stack.len() {
            return;
        }

        let num_iters = self.stack.len() - idx;
        for _ in 0..num_iters {
            let last = self.stack.pop().unwrap();

            let Some(prev_to_last) = self.stack.last_mut() else {
                // current last was the top most in the open blocks stack,
                // since there is no previous to the last
                self.doc.push(last);
                return;
            };

            match prev_to_last {
                Block::BlockQuote(children) => {
                    children.push(last);
                }
                _ => {
                    println!("{:?}", self.stack);
                    println!("{:?}", last);
                    unreachable!("encountered an unintended block as a parent")
                }
            }
        }
    }
}

impl<'a> Into<document::Document<'a>> for Parser<'a> {
    fn into(self) -> document::Document<'a> {
        document::Document::from(self.doc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Case<'a> {
        name: &'static str,
        stack: Vec<Block<'a>>,
        input: &'static str,
        expected: Option<Block<'a>>,
    }

    #[test]
    fn test_try_open_indented_code() {
        let tests = vec![
            Case {
                name: "does_not_interrupt_open_paragraph",
                stack: vec![Block::Paragraph(InlineContent::Raw(vec!["some string"]))],
                input: "    let abc = 'some var'",
                expected: None,
            },
            Case {
                name: "opens_after_non_paragraph_block",
                stack: vec![Block::ThematicBreak],
                input: "    let abc = 'some var'",
                expected: Some(Block::IndentedCode(IndentedCode {
                    content: InlineContent::Raw(vec!["let abc = 'some var'"]),
                })),
            },
            Case {
                name: "opens_at_with_four_space_prefix",
                stack: Vec::new(),
                input: "    let abc = 'some var'",
                expected: Some(Block::IndentedCode(IndentedCode {
                    content: InlineContent::Raw(vec!["let abc = 'some var'"]),
                })),
            },
            Case {
                name: "opens_at_with_tab_prefix",
                stack: Vec::new(),
                input: "\ta = np.array()",
                expected: Some(Block::IndentedCode(IndentedCode {
                    content: InlineContent::Raw(vec!["a = np.array()"]),
                })),
            },
            Case {
                name: "leaves_spaces_after_four_space_prefix",
                stack: Vec::new(),
                input: "      a = np.array()",
                expected: Some(Block::IndentedCode(IndentedCode {
                    content: InlineContent::Raw(vec!["  a = np.array()"]),
                })),
            },
            Case {
                name: "leaves_spaces_after_tab",
                stack: Vec::new(),
                input: "\t  a = np.array()",
                expected: Some(Block::IndentedCode(IndentedCode {
                    content: InlineContent::Raw(vec!["  a = np.array()"]),
                })),
            },
            Case {
                name: "insufficient_spaces",
                stack: Vec::new(),
                input: "   fff",
                expected: None,
            },
            Case {
                name: "no_spaces",
                stack: Vec::new(),
                input: "dfsf",
                expected: None,
            },
        ];

        let mut parser = Parser::new();
        for test in tests {
            parser.stack = test.stack;
            let output = parser.try_open_idented_code(test.input);
            assert_eq!(output, test.expected, "case: {}", test.name);
        }
    }

    #[test]
    fn test_try_open_thematic_break() {
        let tests = vec![
            Case {
                name: "few_markers",
                stack: Vec::new(),
                input: "--",
                expected: None,
            },
            Case {
                name: "several_marker_types",
                stack: Vec::new(),
                input: "-*-",
                expected: None,
            },
            Case {
                name: "invalid_marker",
                stack: Vec::new(),
                input: "+++",
                expected: None,
            },
            Case {
                name: "does_not_interrupt_indented_code",
                stack: Vec::new(),
                input: "    ---",
                expected: None,
            },
            Case {
                name: "does_not_interrupt_tab_prefixed_indented_code",
                stack: Vec::new(),
                input: "\t---",
                expected: None,
            },
            Case {
                name: "tab_and_spaces_ident",
                stack: Vec::new(),
                input: " \t---",
                expected: None,
            },
            Case {
                name: "invalid_characters_after",
                stack: Vec::new(),
                input: "--- ds",
                expected: None,
            },
            Case {
                name: "invalid_characters_before",
                stack: Vec::new(),
                input: "ewr---",
                expected: None,
            },
            Case {
                name: "invalid_characters_in_between",
                stack: Vec::new(),
                input: "-f-f-",
                expected: None,
            },
            Case {
                name: "hyphen_markers",
                stack: Vec::new(),
                input: "---",
                expected: Some(Block::ThematicBreak),
            },
            Case {
                name: "asterisk_markers",
                stack: Vec::new(),
                input: "***",
                expected: Some(Block::ThematicBreak),
            },
            Case {
                name: "underline_markers",
                stack: Vec::new(),
                input: "___",
                expected: Some(Block::ThematicBreak),
            },
            Case {
                name: "more_than_three_marker_occurrences",
                stack: Vec::new(),
                input: "_____________________________________",
                expected: Some(Block::ThematicBreak),
            },
            Case {
                name: "spaces_and_tabs_in_between",
                stack: Vec::new(),
                input: " **  * **\t* ** * **\t   ",
                expected: Some(Block::ThematicBreak),
            },
        ];

        let mut parser = Parser::new();
        for test in tests {
            parser.stack = test.stack;
            let output = parser.try_open_thematic_break(test.input);
            assert_eq!(output, test.expected, "case: {}", test.name);
        }
    }

    #[test]
    fn test_try_open_atx_heading() {
        let tests = vec![
            Case {
                name: "escape_marker_before",
                stack: Vec::new(),
                input: "\\### foo",
                expected: None,
            },
            Case {
                name: "escape_marker_inside",
                stack: Vec::new(),
                input: "##\\# foo",
                expected: None,
            },
            Case {
                name: "too_many_markers",
                stack: Vec::new(),
                input: "######### foo",
                expected: None,
            },
            Case {
                name: "too_many_spaces_before_markers",
                stack: Vec::new(),
                input: "    # foo",
                expected: None,
            },
            Case {
                name: "tab_before_markers",
                stack: Vec::new(),
                input: "\t# foo",
                expected: None,
            },
            Case {
                name: "spaces_and_tab_before_markers",
                stack: Vec::new(),
                input: "  \t# foo",
                expected: None,
            },
            Case {
                name: "h1_heading",
                stack: Vec::new(),
                input: "# foo",
                expected: Some(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo"]),
                    level: ATXHeadingLevel::H1,
                })),
            },
            Case {
                name: "h2_heading",
                stack: Vec::new(),
                input: "## foo",
                expected: Some(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo"]),
                    level: ATXHeadingLevel::H2,
                })),
            },
            Case {
                name: "h3_heading",
                stack: Vec::new(),
                input: "### foo",
                expected: Some(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo"]),
                    level: ATXHeadingLevel::H3,
                })),
            },
            Case {
                name: "h4_heading",
                stack: Vec::new(),
                input: "#### foo",
                expected: Some(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo"]),
                    level: ATXHeadingLevel::H4,
                })),
            },
            Case {
                name: "h5_heading",
                stack: Vec::new(),
                input: "##### foo",
                expected: Some(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo"]),
                    level: ATXHeadingLevel::H5,
                })),
            },
            Case {
                name: "h6_heading",
                stack: Vec::new(),
                input: "###### foo",
                expected: Some(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo"]),
                    level: ATXHeadingLevel::H6,
                })),
            },
            Case {
                name: "empty_content",
                stack: Vec::new(),
                input: "#",
                expected: Some(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec![""]),
                    level: ATXHeadingLevel::H1,
                })),
            },
            Case {
                name: "space_content",
                stack: Vec::new(),
                input: "## ",
                expected: Some(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec![""]),
                    level: ATXHeadingLevel::H2,
                })),
            },
            Case {
                name: "tab_content",
                stack: Vec::new(),
                input: "###\t",
                expected: Some(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec![""]),
                    level: ATXHeadingLevel::H3,
                })),
            },
            Case {
                name: "spaces_around",
                stack: Vec::new(),
                input: "######                  foo                     ",
                expected: Some(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo"]),
                    level: ATXHeadingLevel::H6,
                })),
            },
            Case {
                name: "closing_sequence",
                stack: Vec::new(),
                input: "## foo ##",
                expected: Some(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo"]),
                    level: ATXHeadingLevel::H2,
                })),
            },
            Case {
                name: "not_a_closing_sequence",
                stack: Vec::new(),
                input: "## foo ## b",
                expected: Some(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo ## b"]),
                    level: ATXHeadingLevel::H2,
                })),
            },
            Case {
                name: "escaped_closing_sequence_inside",
                stack: Vec::new(),
                input: "## foo #\\##",
                expected: Some(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo #\\##"]),
                    level: ATXHeadingLevel::H2,
                })),
            },
            Case {
                name: "escaped_closing_sequence_before",
                stack: Vec::new(),
                input: "## foo \\###",
                expected: Some(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo \\###"]),
                    level: ATXHeadingLevel::H2,
                })),
            },
            Case {
                name: "longer_closing_sequence",
                stack: Vec::new(),
                input: "# foo ##################################",
                expected: Some(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo"]),
                    level: ATXHeadingLevel::H1,
                })),
            },
            Case {
                name: "shorter_closing_sequence",
                stack: Vec::new(),
                input: "##### foo ##",
                expected: Some(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo"]),
                    level: ATXHeadingLevel::H5,
                })),
            },
            Case {
                name: "spaces_after_closing_sequence",
                stack: Vec::new(),
                input: "### foo ###     ",
                expected: Some(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo"]),
                    level: ATXHeadingLevel::H3,
                })),
            },
            Case {
                name: "tabs_after_closing_sequence",
                stack: Vec::new(),
                input: "### foo ###     ",
                expected: Some(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo"]),
                    level: ATXHeadingLevel::H3,
                })),
            },
            Case {
                name: "closing_sequence_without_a_space",
                stack: Vec::new(),
                input: "### foo#",
                expected: Some(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo#"]),
                    level: ATXHeadingLevel::H3,
                })),
            },
            Case {
                name: "closing_sequence_after_tab",
                stack: Vec::new(),
                input: "### foo\t#",
                expected: Some(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo"]),
                    level: ATXHeadingLevel::H3,
                })),
            },
            Case {
                name: "three_spaces_before_markers",
                stack: Vec::new(),
                input: "   # foo",
                expected: Some(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo"]),
                    level: ATXHeadingLevel::H1,
                })),
            },
        ];

        let mut parser = Parser::new();
        for test in tests {
            parser.stack = test.stack;
            let output = parser.try_open_atx_heading(test.input);
            assert_eq!(output, test.expected, "case: {}", test.name);
        }
    }

    #[test]
    fn test_try_open_fenced_code() {
        let tests = vec![
            Case {
                name: "too_many_spaces_before_markers",
                stack: Vec::new(),
                input: "    ~~~",
                expected: None,
            },
            Case {
                name: "tab_before_markers",
                stack: Vec::new(),
                input: "\t```",
                expected: None,
            },
            Case {
                name: "tab_and_spaces_before_markers",
                stack: Vec::new(),
                input: " \t ~~~",
                expected: None,
            },
            Case {
                name: "too_few_markers",
                stack: Vec::new(),
                input: "``",
                expected: None,
            },
            Case {
                name: "tildes",
                stack: Vec::new(),
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
                stack: Vec::new(),
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
                stack: Vec::new(),
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
                stack: Vec::new(),
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
                stack: Vec::new(),
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
                stack: Vec::new(),
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
                stack: Vec::new(),
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
                stack: Vec::new(),
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

        let mut parser = Parser::new();
        for test in tests {
            parser.stack = test.stack;
            let output = parser.try_open_fenced_code(test.input);
            assert_eq!(output, test.expected, "case: {}", test.name);
        }
    }
}
