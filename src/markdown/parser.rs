pub mod errors;
use crate::markdown::document;

use super::block::*;

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

    fn parse(&mut self, input: &'a str) -> Result<document::Document<'a>, errors::ParserError> {
        for line in input.lines() {
            self.process_line(line);
        }

        self.close_from(0);

        Ok(self.doc.clone())
    }

    fn process_line(&mut self, line: &'a str) {
        let mut continuation = line;
        let mut last_open_idx = 0;
        for block in self.stack.iter() {
            if let Some(s) = self.try_continue(block, continuation) {
                continuation = s;
                last_open_idx += 1;
            } else {
                break;
            }
        }

        self.close_from(last_open_idx);

        if let Some(new_block) = self.try_open(continuation) {
            self.stack.push(new_block);
        }
    }

    fn try_continue(&self, block: &Block, line: &'a str) -> Option<&'a str> {
        match block {
            Block::ThematicBreak => None,
            Block::ATXHeading(_) => None,
            Block::IndentedCode(_) => line.strip_prefix("    "),
            Block::FencedCode(_) => Some(line),
            Block::Paragraph(_) => match line.trim() {
                "" => None,
                _ => Some(line),
            },

            Block::BlockQuote(_) => line.strip_prefix("> "),
        }
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

        None
    }

    fn try_open_idented_code(&self, line: &'a str) -> Option<Block<'a>> {
        if let Some(Block::Paragraph(_)) = self.stack.iter().last() {
            return None;
        }
        if let Some(s) = line.strip_prefix("    ") {
            return Some(Block::IndentedCode(InlineContent::Raw(vec![s])));
        }
        if let Some(s) = line.strip_prefix("\t") {
            return Some(Block::IndentedCode(InlineContent::Raw(vec![s])));
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

    // fn try_open_fenced_code(&self, line: &'a str) -> Option<Block<'a>> {
    //     None
    // }

    /// Closes open blocks on the stack starting from particular index
    fn close_from(&mut self, idx: usize) {
        if idx == self.stack.len() {
            return;
        }

        let mut head = self.stack.pop().unwrap();
        for _ in self.stack.len()..idx {
            let next = head;
            head = self.stack.pop().unwrap();

            match head {
                Block::BlockQuote(children) => {
                    let mut children = children.clone();
                    children.push(next);
                    head = Block::BlockQuote(children);
                }
                _ => unreachable!(),
            }
        }

        if idx == 0 {
            self.doc.push(head);
            return;
        }

        match self.stack.pop().unwrap() {
            Block::BlockQuote(children) => {
                let mut children = children.clone();
                children.push(head);
                self.stack.push(Block::BlockQuote(children))
            }
            _ => unreachable!(),
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
                expected: Some(Block::IndentedCode(InlineContent::Raw(vec![
                    "let abc = 'some var'",
                ]))),
            },
            Case {
                name: "opens_at_with_four_space_prefix",
                stack: Vec::new(),
                input: "    let abc = 'some var'",
                expected: Some(Block::IndentedCode(InlineContent::Raw(vec![
                    "let abc = 'some var'",
                ]))),
            },
            Case {
                name: "opens_at_with_tab_prefix",
                stack: Vec::new(),
                input: "\ta = np.array()",
                expected: Some(Block::IndentedCode(InlineContent::Raw(vec![
                    "a = np.array()",
                ]))),
            },
            Case {
                name: "leaves_spaces_after_four_space_prefix",
                stack: Vec::new(),
                input: "      a = np.array()",
                expected: Some(Block::IndentedCode(InlineContent::Raw(vec![
                    "  a = np.array()",
                ]))),
            },
            Case {
                name: "leaves_spaces_after_tab",
                stack: Vec::new(),
                input: "\t  a = np.array()",
                expected: Some(Block::IndentedCode(InlineContent::Raw(vec![
                    "  a = np.array()",
                ]))),
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

    // #[test]
    // fn test_try_open_fenced_code() {
    //     let tests = vec![
    //         Case {
    //             name: "few_markers",
    //             stack: Vec::new(),
    //             input: "--",
    //             expected: None,
    //         },
    //         Case {
    //             name: "several_marker_types",
    //             stack: Vec::new(),
    //             input: "-*-",
    //             expected: None,
    //         },
    //     ];

    //     let mut parser = Parser::new();
    //     for test in tests {
    //         parser.stack = test.stack;
    //         let output = parser.try_open_fenced_code(test.input);
    //         assert_eq!(output, test.expected, "case: {}", test.name);
    //     }
    // }
}
