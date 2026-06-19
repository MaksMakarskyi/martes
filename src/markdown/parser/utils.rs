use crate::markdown::block::ListType;

/// Strips up to 3 spaces of leading indentation from a line.
///
/// Returns `None` if the line contains a tab before content or has 4 or
/// more leading spaces, as these are not valid block-level indentation
/// for majority of blocks. Indented code uses different check.
pub fn strip_indent<'a>(line: &'a str) -> Option<&'a str> {
    let mut ident_idx = 0;
    for (idx, ch) in line.char_indices() {
        match ch {
            '\t' => return None,
            ' ' => {
                if idx >= 3 {
                    return None;
                }
            }
            _ => {
                ident_idx = idx;
                break;
            }
        }
    }

    Some(&line[ident_idx..])
}

/// Parses the list marker, returns the type of the list and continuation
pub fn parse_list_marker<'a>(line: &'a str) -> Option<(ListType, &'a str)> {
    let mut list_type: Option<ListType> = None;
    let mut order: Option<u32> = None;
    let mut digit_count: u8 = 0;
    let mut continuation = "";
    for (idx, ch) in line.char_indices() {
        match ch {
            '-' | '+' | '*' => {
                if list_type.is_some() || order.is_some() {
                    return None;
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
                    return None;
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
                    return None;
                }

                order = match order {
                    Some(num) => Some(num * 10 + ch as u32 - '0' as u32),
                    None => Some(ch as u32 - '0' as u32),
                }
            }
            ' ' | '\t' => {
                continuation = &line[idx + 1..];
                break;
            }
            _ => return None,
        }
    }

    list_type.map(|lt| (lt, continuation))
}
