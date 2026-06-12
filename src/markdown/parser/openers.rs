use super::super::block::*;

pub fn try_open<'a>(line: &'a str, last: Option<&Block<'a>>) -> OpenResult<'a> {
    if let OpenResult::Opened(b) = try_open_idented_code(line, last) {
        return OpenResult::Opened(b);
    }

    // TODO: check for the Setext Heading

    if let OpenResult::Opened(b) = try_open_thematic_break(line, last) {
        return OpenResult::Opened(b);
    }
    if let OpenResult::Opened(b) = try_open_atx_heading(line, last) {
        return OpenResult::Opened(b);
    }
    if let OpenResult::Opened(b) = try_open_fenced_code(line, last) {
        return OpenResult::Opened(b);
    }
    if let OpenResult::Opened(b) = try_open_link_reference(line, last) {
        return OpenResult::Opened(b);
    }

    if let OpenResult::Continue(b, s) = try_open_block_quote(line, last) {
        return OpenResult::Continue(b, s);
    }
    if let OpenResult::Continue(b, s) = try_open_list(line, last) {
        return OpenResult::Continue(b, s);
    }

    OpenResult::NotOpened
}

fn try_open_idented_code<'a>(line: &'a str, last: Option<&Block<'a>>) -> OpenResult<'a> {
    if let Some(Block::Paragraph(_)) = last {
        return OpenResult::NotOpened;
    }
    if let Some(Block::FencedCode(_)) = last {
        return OpenResult::NotOpened;
    }

    if let Some(s) = line.strip_prefix("    ") {
        return OpenResult::Opened(Block::IndentedCode(IndentedCode {
            content: InlineContent::Raw(vec![s]),
        }));
    }
    if let Some(s) = line.strip_prefix("\t") {
        return OpenResult::Opened(Block::IndentedCode(IndentedCode {
            content: InlineContent::Raw(vec![s]),
        }));
    }

    return OpenResult::NotOpened;
}

fn try_open_thematic_break<'a>(line: &'a str, last: Option<&Block<'a>>) -> OpenResult<'a> {
    if let Some(Block::FencedCode(_)) = last {
        return OpenResult::NotOpened;
    }

    let after_space_indent = line.trim_start_matches(' ');
    if line.len() - after_space_indent.len() > 3 {
        return OpenResult::NotOpened;
    }

    let after_indent = after_space_indent.trim_start_matches('\t');
    if after_space_indent.len() > after_indent.len() {
        return OpenResult::NotOpened;
    }

    let mut thematic_ch = None;
    let mut occ = 0;
    for ch in after_indent.chars() {
        match ch {
            '-' | '_' | '*' => {
                if let Some(thematic_ch) = thematic_ch {
                    if thematic_ch != ch {
                        return OpenResult::NotOpened;
                    } else {
                        occ += 1;
                    }
                } else {
                    thematic_ch = Some(ch);
                    occ += 1;
                }
            }
            '\t' | ' ' => continue,
            _ => return OpenResult::NotOpened,
        }
    }

    if occ < 3 {
        return OpenResult::NotOpened;
    }

    OpenResult::Opened(Block::ThematicBreak)
}

fn try_open_atx_heading<'a>(line: &'a str, last: Option<&Block<'a>>) -> OpenResult<'a> {
    if let Some(Block::FencedCode(_)) = last {
        return OpenResult::NotOpened;
    }

    let after_indent = line.trim_start_matches(' ');
    if line.len() - after_indent.len() > 3 {
        return OpenResult::NotOpened;
    }

    let after_markers = after_indent.trim_start_matches('#');
    let level = after_indent.len() - after_markers.len();
    if level <= 0 || level > 6 {
        return OpenResult::NotOpened;
    }

    let Ok(level) = ATXHeadingLevel::try_from(level as u8) else {
        return OpenResult::NotOpened;
    };

    let after_whitespaces_before = after_markers.trim_start();
    if after_markers.len() == after_whitespaces_before.len() {
        if after_whitespaces_before.len() == 0 {
            return OpenResult::Opened(Block::ATXHeading(ATXHeading {
                content: InlineContent::Raw(vec![""]),
                level,
            }));
        }

        return OpenResult::NotOpened;
    }

    let after_whitespaces = after_whitespaces_before.trim_end();
    let after_closing_seq = after_whitespaces.trim_end_matches('#');
    if after_closing_seq.len() == after_whitespaces.len() {
        return OpenResult::Opened(Block::ATXHeading(ATXHeading {
            content: InlineContent::Raw(vec![after_closing_seq]),
            level,
        }));
    }

    let after_whitespaces_after = after_closing_seq.trim_end();
    if after_whitespaces_after.len() == after_closing_seq.len() {
        return OpenResult::Opened(Block::ATXHeading(ATXHeading {
            content: InlineContent::Raw(vec![after_whitespaces]),
            level,
        }));
    }

    OpenResult::Opened(Block::ATXHeading(ATXHeading {
        content: InlineContent::Raw(vec![after_whitespaces_after]),
        level,
    }))
}

fn try_open_fenced_code<'a>(line: &'a str, last: Option<&Block<'a>>) -> OpenResult<'a> {
    let after_indent = line.trim_start_matches(' ');
    let indent_size = line.len() - after_indent.len();
    if indent_size > 3 {
        return OpenResult::NotOpened;
    }

    let after_tabs = after_indent.trim_start_matches('\t');
    if after_tabs.len() < after_indent.len() {
        return OpenResult::NotOpened;
    }

    let fence_type: FenceType;
    let fence_occ: usize;
    let mut after_fence = after_tabs.trim_start_matches('~');
    if after_fence.len() == after_tabs.len() {
        after_fence = after_tabs.trim_start_matches('`');
        if after_fence.len() == after_tabs.len() {
            return OpenResult::NotOpened;
        }

        fence_type = FenceType::Backtick;
    } else {
        fence_type = FenceType::Tilda;
    }

    fence_occ = after_tabs.len() - after_fence.len();
    if fence_occ <= 2 {
        return OpenResult::NotOpened;
    }

    if let Some(Block::FencedCode(fc)) = last {
        if fc.fence_occ > fence_occ || fc.fence_type != fence_type {
            return OpenResult::NotOpened;
        }
    }

    let language = after_fence.trim_start().split(' ').next().unwrap_or("");

    OpenResult::Opened(Block::FencedCode(FencedCode {
        content: InlineContent::Raw(Vec::new()),
        language: language,
        ident: indent_size,
        fence_type: fence_type,
        fence_occ: fence_occ,
    }))
}

fn try_open_link_reference<'a>(line: &'a str, last: Option<&Block<'a>>) -> OpenResult<'a> {
    if let Some(Block::Paragraph(_)) = last {
        return OpenResult::NotOpened;
    }
    if let Some(Block::FencedCode(_)) = last {
        return OpenResult::NotOpened;
    }

    let after_indent = line.trim_start_matches(' ');
    let indent_size = line.len() - after_indent.len();
    if indent_size > 3 {
        return OpenResult::NotOpened;
    }

    let after_tabs = after_indent.trim_start_matches('\t');
    if after_tabs.len() < after_indent.len() {
        return OpenResult::NotOpened;
    }

    // TODO: Finish the main logic

    OpenResult::NotOpened
}

fn try_open_block_quote<'a>(line: &'a str, last: Option<&Block<'a>>) -> OpenResult<'a> {
    if let Some(Block::FencedCode(_)) = last {
        return OpenResult::NotOpened;
    }

    let after_indent = line.trim_start_matches(' ');
    let indent_size = line.len() - after_indent.len();
    if indent_size > 3 {
        return OpenResult::NotOpened;
    }

    let after_tabs = after_indent.trim_start_matches('\t');
    if after_tabs.len() < after_indent.len() {
        return OpenResult::NotOpened;
    }

    let Some(after_marker) = after_tabs.strip_prefix('>') else {
        return OpenResult::NotOpened;
    };
    let after_space = after_marker.strip_prefix(' ').unwrap_or(after_marker);

    OpenResult::Continue(
        Block::BlockQuote(BlockQuote {
            children: Vec::new(),
        }),
        after_space,
    )
}

fn try_open_list<'a>(line: &'a str, last: Option<&Block<'a>>) -> OpenResult<'a> {
    if let Some(Block::FencedCode(_)) = last {
        return OpenResult::NotOpened;
    }

    let mut ident_idx = 0;
    for (idx, ch) in line.char_indices() {
        match ch {
            '\t' => return OpenResult::NotOpened,
            ' ' => {
                if idx >= 3 {
                    return OpenResult::NotOpened;
                }
            }
            _ => {
                ident_idx = idx;
                break;
            }
        }
    }

    let mut list_type: Option<ListType> = None;
    let mut order: Option<u32> = None;
    let mut digit_count: u8 = 0;
    let mut continuation = "";
    for (idx, ch) in line[ident_idx..].char_indices() {
        match ch {
            '-' | '+' | '*' => {
                if list_type.is_some() || order.is_some() {
                    return OpenResult::NotOpened;
                }

                list_type = match ch {
                    '-' => Some(ListType::UnorderedMinus),
                    '+' => Some(ListType::UnorderedPlus),
                    '*' => Some(ListType::UnorderedAsterisk),
                    _ => unreachable!("other bytes are filtered by outer arm"),
                };
            }
            '.' | ')' => {
                if list_type.is_some() || order.is_none() {
                    return OpenResult::NotOpened;
                }

                list_type = match ch {
                    '.' => Some(ListType::OrderedDot(order.unwrap())),
                    ')' => Some(ListType::OrdererParentheses(order.unwrap())),
                    _ => unreachable!("other bytes are filtered by outer arm"),
                };
            }
            '0'..='9' => {
                digit_count += 1;

                if list_type.is_some() || digit_count >= 10 {
                    return OpenResult::NotOpened;
                }

                order = match order {
                    Some(num) => Some(num * 10 + ch as u32 - '0' as u32),
                    None => Some(ch as u32 - '0' as u32),
                }
            }
            ' ' => {
                continuation = &line[ident_idx + idx + 1..];
                break;
            }
            _ => return OpenResult::NotOpened,
        }
    }

    if let Some(lt) = list_type {
        return OpenResult::Continue(
            Block::List(List {
                items: Vec::new(),
                list_type: lt,
                tight: true,
            }),
            continuation,
        );
    }

    return OpenResult::NotOpened;
}

#[derive(PartialEq, Debug, Clone)]
pub enum OpenResult<'a> {
    Continue(Block<'a>, &'a str),
    Opened(Block<'a>),
    NotOpened,
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Case<'a> {
        name: &'static str,
        input: &'static str,
        expected: OpenResult<'a>,
    }

    #[test]
    fn test_try_open_indented_code() {
        let tests = vec![
            Case {
                name: "opens_at_with_four_space_prefix",
                input: "    let abc = 'some var'",
                expected: OpenResult::Opened(Block::IndentedCode(IndentedCode {
                    content: InlineContent::Raw(vec!["let abc = 'some var'"]),
                })),
            },
            Case {
                name: "opens_at_with_tab_prefix",
                input: "\ta = np.array()",
                expected: OpenResult::Opened(Block::IndentedCode(IndentedCode {
                    content: InlineContent::Raw(vec!["a = np.array()"]),
                })),
            },
            Case {
                name: "leaves_spaces_after_four_space_prefix",
                input: "      a = np.array()",
                expected: OpenResult::Opened(Block::IndentedCode(IndentedCode {
                    content: InlineContent::Raw(vec!["  a = np.array()"]),
                })),
            },
            Case {
                name: "leaves_spaces_after_tab",
                input: "\t  a = np.array()",
                expected: OpenResult::Opened(Block::IndentedCode(IndentedCode {
                    content: InlineContent::Raw(vec!["  a = np.array()"]),
                })),
            },
            Case {
                name: "insufficient_spaces",
                input: "   fff",
                expected: OpenResult::NotOpened,
            },
            Case {
                name: "no_spaces",
                input: "dfsf",
                expected: OpenResult::NotOpened,
            },
        ];

        for test in tests {
            assert_eq!(
                try_open_idented_code(test.input, None),
                test.expected,
                "case: {}",
                test.name
            );
        }
    }

    #[test]
    fn test_try_open_thematic_break() {
        let tests = vec![
            Case {
                name: "few_markers",
                input: "--",
                expected: OpenResult::NotOpened,
            },
            Case {
                name: "several_marker_types",
                input: "-*-",
                expected: OpenResult::NotOpened,
            },
            Case {
                name: "invalid_marker",
                input: "+++",
                expected: OpenResult::NotOpened,
            },
            Case {
                name: "does_not_interrupt_indented_code",
                input: "    ---",
                expected: OpenResult::NotOpened,
            },
            Case {
                name: "does_not_interrupt_tab_prefixed_indented_code",
                input: "\t---",
                expected: OpenResult::NotOpened,
            },
            Case {
                name: "tab_and_spaces_ident",
                input: " \t---",
                expected: OpenResult::NotOpened,
            },
            Case {
                name: "invalid_characters_after",
                input: "--- ds",
                expected: OpenResult::NotOpened,
            },
            Case {
                name: "invalid_characters_before",
                input: "ewr---",
                expected: OpenResult::NotOpened,
            },
            Case {
                name: "invalid_characters_in_between",
                input: "-f-f-",
                expected: OpenResult::NotOpened,
            },
            Case {
                name: "hyphen_markers",
                input: "---",
                expected: OpenResult::Opened(Block::ThematicBreak),
            },
            Case {
                name: "asterisk_markers",
                input: "***",
                expected: OpenResult::Opened(Block::ThematicBreak),
            },
            Case {
                name: "underline_markers",
                input: "___",
                expected: OpenResult::Opened(Block::ThematicBreak),
            },
            Case {
                name: "more_than_three_marker_occurrences",
                input: "_____________________________________",
                expected: OpenResult::Opened(Block::ThematicBreak),
            },
            Case {
                name: "spaces_and_tabs_in_between",
                input: " **  * **\t* ** * **\t   ",
                expected: OpenResult::Opened(Block::ThematicBreak),
            },
        ];

        for test in tests {
            assert_eq!(
                try_open_thematic_break(test.input, None),
                test.expected,
                "case: {}",
                test.name
            );
        }
    }

    #[test]
    fn test_try_open_atx_heading() {
        let tests = vec![
            Case {
                name: "escape_marker_before",
                input: "\\### foo",
                expected: OpenResult::NotOpened,
            },
            Case {
                name: "escape_marker_inside",
                input: "##\\# foo",
                expected: OpenResult::NotOpened,
            },
            Case {
                name: "too_many_markers",
                input: "######### foo",
                expected: OpenResult::NotOpened,
            },
            Case {
                name: "too_many_spaces_before_markers",
                input: "    # foo",
                expected: OpenResult::NotOpened,
            },
            Case {
                name: "tab_before_markers",
                input: "\t# foo",
                expected: OpenResult::NotOpened,
            },
            Case {
                name: "spaces_and_tab_before_markers",
                input: "  \t# foo",
                expected: OpenResult::NotOpened,
            },
            Case {
                name: "h1_heading",
                input: "# foo",
                expected: OpenResult::Opened(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo"]),
                    level: ATXHeadingLevel::H1,
                })),
            },
            Case {
                name: "h2_heading",
                input: "## foo",
                expected: OpenResult::Opened(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo"]),
                    level: ATXHeadingLevel::H2,
                })),
            },
            Case {
                name: "h3_heading",
                input: "### foo",
                expected: OpenResult::Opened(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo"]),
                    level: ATXHeadingLevel::H3,
                })),
            },
            Case {
                name: "h4_heading",
                input: "#### foo",
                expected: OpenResult::Opened(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo"]),
                    level: ATXHeadingLevel::H4,
                })),
            },
            Case {
                name: "h5_heading",
                input: "##### foo",
                expected: OpenResult::Opened(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo"]),
                    level: ATXHeadingLevel::H5,
                })),
            },
            Case {
                name: "h6_heading",
                input: "###### foo",
                expected: OpenResult::Opened(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo"]),
                    level: ATXHeadingLevel::H6,
                })),
            },
            Case {
                name: "empty_content",
                input: "#",
                expected: OpenResult::Opened(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec![""]),
                    level: ATXHeadingLevel::H1,
                })),
            },
            Case {
                name: "space_content",
                input: "## ",
                expected: OpenResult::Opened(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec![""]),
                    level: ATXHeadingLevel::H2,
                })),
            },
            Case {
                name: "tab_content",
                input: "###\t",
                expected: OpenResult::Opened(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec![""]),
                    level: ATXHeadingLevel::H3,
                })),
            },
            Case {
                name: "spaces_around",
                input: "######                  foo                     ",
                expected: OpenResult::Opened(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo"]),
                    level: ATXHeadingLevel::H6,
                })),
            },
            Case {
                name: "closing_sequence",
                input: "## foo ##",
                expected: OpenResult::Opened(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo"]),
                    level: ATXHeadingLevel::H2,
                })),
            },
            Case {
                name: "not_a_closing_sequence",
                input: "## foo ## b",
                expected: OpenResult::Opened(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo ## b"]),
                    level: ATXHeadingLevel::H2,
                })),
            },
            Case {
                name: "escaped_closing_sequence_inside",
                input: "## foo #\\##",
                expected: OpenResult::Opened(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo #\\##"]),
                    level: ATXHeadingLevel::H2,
                })),
            },
            Case {
                name: "escaped_closing_sequence_before",
                input: "## foo \\###",
                expected: OpenResult::Opened(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo \\###"]),
                    level: ATXHeadingLevel::H2,
                })),
            },
            Case {
                name: "longer_closing_sequence",
                input: "# foo ##################################",
                expected: OpenResult::Opened(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo"]),
                    level: ATXHeadingLevel::H1,
                })),
            },
            Case {
                name: "shorter_closing_sequence",
                input: "##### foo ##",
                expected: OpenResult::Opened(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo"]),
                    level: ATXHeadingLevel::H5,
                })),
            },
            Case {
                name: "spaces_after_closing_sequence",
                input: "### foo ###     ",
                expected: OpenResult::Opened(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo"]),
                    level: ATXHeadingLevel::H3,
                })),
            },
            Case {
                name: "tabs_after_closing_sequence",
                input: "### foo ###     ",
                expected: OpenResult::Opened(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo"]),
                    level: ATXHeadingLevel::H3,
                })),
            },
            Case {
                name: "closing_sequence_without_a_space",
                input: "### foo#",
                expected: OpenResult::Opened(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo#"]),
                    level: ATXHeadingLevel::H3,
                })),
            },
            Case {
                name: "closing_sequence_after_tab",
                input: "### foo\t#",
                expected: OpenResult::Opened(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo"]),
                    level: ATXHeadingLevel::H3,
                })),
            },
            Case {
                name: "three_spaces_before_markers",
                input: "   # foo",
                expected: OpenResult::Opened(Block::ATXHeading(ATXHeading {
                    content: InlineContent::Raw(vec!["foo"]),
                    level: ATXHeadingLevel::H1,
                })),
            },
        ];

        for test in tests {
            assert_eq!(
                try_open_atx_heading(test.input, None),
                test.expected,
                "case: {}",
                test.name
            );
        }
    }

    #[test]
    fn test_try_open_fenced_code() {
        let tests = vec![
            Case {
                name: "too_many_spaces_before_markers",
                input: "    ~~~",
                expected: OpenResult::NotOpened,
            },
            Case {
                name: "tab_before_markers",
                input: "\t```",
                expected: OpenResult::NotOpened,
            },
            Case {
                name: "tab_and_spaces_before_markers",
                input: " \t ~~~",
                expected: OpenResult::NotOpened,
            },
            Case {
                name: "too_few_markers",
                input: "``",
                expected: OpenResult::NotOpened,
            },
            Case {
                name: "tildes",
                input: "~~~",
                expected: OpenResult::Opened(Block::FencedCode(FencedCode {
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
                expected: OpenResult::Opened(Block::FencedCode(FencedCode {
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
                expected: OpenResult::Opened(Block::FencedCode(FencedCode {
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
                expected: OpenResult::Opened(Block::FencedCode(FencedCode {
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
                expected: OpenResult::Opened(Block::FencedCode(FencedCode {
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
                expected: OpenResult::Opened(Block::FencedCode(FencedCode {
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
                expected: OpenResult::Opened(Block::FencedCode(FencedCode {
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
                expected: OpenResult::Opened(Block::FencedCode(FencedCode {
                    content: InlineContent::Raw(Vec::new()),
                    language: "rust",
                    ident: 1,
                    fence_type: FenceType::Tilda,
                    fence_occ: 4,
                })),
            },
        ];

        for test in tests {
            assert_eq!(
                try_open_fenced_code(test.input, None),
                test.expected,
                "case: {}",
                test.name
            );
        }
    }

    #[test]
    fn test_try_open_block_quote() {
        let tests = vec![
            Case {
                name: "empty_line",
                input: "",
                expected: OpenResult::NotOpened,
            },
            Case {
                name: "no_marker",
                input: "some text",
                expected: OpenResult::NotOpened,
            },
            Case {
                name: "too_many_spaces",
                input: "    > some text",
                expected: OpenResult::NotOpened,
            },
            Case {
                name: "tab_before",
                input: "\t> some text",
                expected: OpenResult::NotOpened,
            },
            Case {
                name: "tab_and_spaces_before",
                input: " \t > some text",
                expected: OpenResult::NotOpened,
            },
            Case {
                name: "simple_quote",
                input: "> some text",
                expected: OpenResult::Continue(
                    Block::BlockQuote(BlockQuote {
                        children: Vec::new(),
                    }),
                    "some text",
                ),
            },
            Case {
                name: "no_space_after_marker",
                input: ">some text",
                expected: OpenResult::Continue(
                    Block::BlockQuote(BlockQuote {
                        children: Vec::new(),
                    }),
                    "some text",
                ),
            },
            Case {
                name: "three_spaces_after_markers",
                input: ">   some text",
                expected: OpenResult::Continue(
                    Block::BlockQuote(BlockQuote {
                        children: Vec::new(),
                    }),
                    "  some text",
                ),
            },
            Case {
                name: "three_spaces_before_markers",
                input: "   > some text",
                expected: OpenResult::Continue(
                    Block::BlockQuote(BlockQuote {
                        children: Vec::new(),
                    }),
                    "some text",
                ),
            },
        ];

        for test in tests {
            assert_eq!(
                try_open_block_quote(test.input, None),
                test.expected,
                "case: {}",
                test.name
            );
        }
    }

    #[test]
    fn test_try_open_list() {
        let tests = vec![
            Case {
                name: "empty_line",
                input: "",
                expected: OpenResult::NotOpened,
            },
            Case {
                name: "spaces_nly",
                input: "  ",
                expected: OpenResult::NotOpened,
            },
            Case {
                name: "too_many_spaces",
                input: "    - dsf",
                expected: OpenResult::NotOpened,
            },
            Case {
                name: "tab_before_markers",
                input: "\t- dfsd",
                expected: OpenResult::NotOpened,
            },
            Case {
                name: "spaces_and_tab_before_markers",
                input: " \t - sdfsd",
                expected: OpenResult::NotOpened,
            },
            Case {
                name: "spaces_and_tab_before_markers",
                input: " \t - dfsdf",
                expected: OpenResult::NotOpened,
            },
            Case {
                name: "digit_after_dot",
                input: "1.1 dfsdf",
                expected: OpenResult::NotOpened,
            },
            Case {
                name: "minus_after_digits",
                input: "11- dfsdf",
                expected: OpenResult::NotOpened,
            },
            Case {
                name: "asterisk_between_digits",
                input: "1*1 sdfsdf",
                expected: OpenResult::NotOpened,
            },
            Case {
                name: "too_many_digits",
                input: "1234567890. sfsdf",
                expected: OpenResult::NotOpened,
            },
            Case {
                name: "ordered_dot",
                input: "1. some text",
                expected: OpenResult::Continue(
                    Block::List(List {
                        items: Vec::new(),
                        list_type: ListType::OrderedDot(1),
                        tight: true,
                    }),
                    "some text",
                ),
            },
            Case {
                name: "ordered_parentheses",
                input: "1) some text",
                expected: OpenResult::Continue(
                    Block::List(List {
                        items: Vec::new(),
                        list_type: ListType::OrdererParentheses(1),
                        tight: true,
                    }),
                    "some text",
                ),
            },
            Case {
                name: "unordered_asterisk",
                input: "* some text",
                expected: OpenResult::Continue(
                    Block::List(List {
                        items: Vec::new(),
                        list_type: ListType::UnorderedAsterisk,
                        tight: true,
                    }),
                    "some text",
                ),
            },
            Case {
                name: "unordered_minus",
                input: "- some text",
                expected: OpenResult::Continue(
                    Block::List(List {
                        items: Vec::new(),
                        list_type: ListType::UnorderedMinus,
                        tight: true,
                    }),
                    "some text",
                ),
            },
            Case {
                name: "unordered_plus",
                input: "+ some text",
                expected: OpenResult::Continue(
                    Block::List(List {
                        items: Vec::new(),
                        list_type: ListType::UnorderedPlus,
                        tight: true,
                    }),
                    "some text",
                ),
            },
            Case {
                name: "spaces_before_markers",
                input: "   - some text",
                expected: OpenResult::Continue(
                    Block::List(List {
                        items: Vec::new(),
                        list_type: ListType::UnorderedMinus,
                        tight: true,
                    }),
                    "some text",
                ),
            },
            Case {
                name: "spaces_after_markers",
                input: "-      some text",
                expected: OpenResult::Continue(
                    Block::List(List {
                        items: Vec::new(),
                        list_type: ListType::UnorderedMinus,
                        tight: true,
                    }),
                    "     some text",
                ),
            },
            Case {
                name: "tabs_after_markers",
                input: "- \t\tsome text",
                expected: OpenResult::Continue(
                    Block::List(List {
                        items: Vec::new(),
                        list_type: ListType::UnorderedMinus,
                        tight: true,
                    }),
                    "\t\tsome text",
                ),
            },
            Case {
                name: "empty_item_ordered",
                input: "0)",
                expected: OpenResult::Continue(
                    Block::List(List {
                        items: Vec::new(),
                        list_type: ListType::OrdererParentheses(0),
                        tight: true,
                    }),
                    "",
                ),
            },
            Case {
                name: "empty_item_unordered",
                input: "*",
                expected: OpenResult::Continue(
                    Block::List(List {
                        items: Vec::new(),
                        list_type: ListType::UnorderedAsterisk,
                        tight: true,
                    }),
                    "",
                ),
            },
            Case {
                name: "ordered_large_start",
                input: "12045) some text",
                expected: OpenResult::Continue(
                    Block::List(List {
                        items: Vec::new(),
                        list_type: ListType::OrdererParentheses(12045),
                        tight: true,
                    }),
                    "some text",
                ),
            },
            Case {
                name: "zeros_in_front",
                input: "003456789. some text",
                expected: OpenResult::Continue(
                    Block::List(List {
                        items: Vec::new(),
                        list_type: ListType::OrderedDot(3456789),
                        tight: true,
                    }),
                    "some text",
                ),
            },
        ];

        for test in tests {
            assert_eq!(
                try_open_list(test.input, None),
                test.expected,
                "case: {}",
                test.name
            );
        }
    }
}
