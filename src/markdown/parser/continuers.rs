use super::utils::parse_list_marker;
use crate::markdown::block::*;

pub fn try_continue<'a>(block: &Block, line: &'a str) -> ContinueResult<'a> {
    match block {
        Block::ThematicBreak | Block::ATXHeading(_) | Block::Paragraph(_) => {
            ContinueResult::NotContinue
        }
        // Block::LinkReference(_) => unimplemented!(),
        Block::List(list) => try_continue_list(list, line),
        Block::ListItem(li) => try_continue_list_item(li, line),
        Block::IndentedCode(_) => try_continue_indented_code(line),
        Block::FencedCode(fc) => try_continue_fenced_code(fc, line),
        Block::BlockQuote(_) => try_continue_blockquote(line),
    }
}

fn try_continue_indented_code<'a>(line: &'a str) -> ContinueResult<'a> {
    match line.strip_prefix("    ") {
        Some(s) => ContinueResult::Continue(s),
        None => ContinueResult::NotContinue,
    }
}

fn try_continue_fenced_code<'a>(fc: &FencedCode, line: &'a str) -> ContinueResult<'a> {
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

fn try_continue_blockquote<'a>(line: &'a str) -> ContinueResult<'a> {
    match line.strip_prefix(">") {
        Some(after_marker) => match after_marker.strip_prefix(" ") {
            Some(after_space) => ContinueResult::Continue(after_space),
            None => ContinueResult::Continue(after_marker),
        },
        None => ContinueResult::NotContinue,
    }
}

fn try_continue_list<'a>(list: &List, line: &'a str) -> ContinueResult<'a> {
    if let Some((list_type, _)) = parse_list_marker(line)
        && !list_type.same_variant(&list.list_type)
    {
        return ContinueResult::NotContinue;
    }

    ContinueResult::Continue(line)
}

fn try_continue_list_item<'a>(li: &ListItem, line: &'a str) -> ContinueResult<'a> {
    for (idx, ch) in line.char_indices() {
        if idx == li.padding {
            return ContinueResult::Continue(&line[idx..]);
        }

        match ch {
            ' ' | '\n' => continue,
            _ => return ContinueResult::NotContinue,
        }
    }

    return ContinueResult::NotContinue;
}

#[derive(PartialEq, Debug, Clone)]
pub enum ContinueResult<'a> {
    Continue(&'a str),
    NotContinue,
    Close,
}
