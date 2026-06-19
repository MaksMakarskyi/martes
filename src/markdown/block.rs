use super::inline::Inline;

#[derive(PartialEq, Debug, Clone)]
pub enum Block<'a> {
    // Leaf Blocks
    ThematicBreak,
    ATXHeading(ATXHeading<'a>),
    // SetextHeading(&'a str, SetextHeadingLevel),
    IndentedCode(IndentedCode<'a>),
    FencedCode(FencedCode<'a>),
    // HTML(&'a str),
    // LinkReference(LinkReference<'a>),
    Paragraph(InlineContent<'a>),

    // Container Blocks
    BlockQuote(BlockQuote<'a>),
    ListItem(ListItem<'a>),
    List(List<'a>),
}

#[derive(PartialEq, Debug, Clone)]
pub enum InlineContent<'a> {
    Raw(Vec<&'a str>),
    Parsed(Vec<Inline<'a>>),
}

#[derive(PartialEq, Debug, Clone)]
pub struct ATXHeading<'a> {
    pub content: InlineContent<'a>,
    pub level: ATXHeadingLevel,
}

#[derive(PartialEq, Debug, Clone)]
pub enum ATXHeadingLevel {
    H1,
    H2,
    H3,
    H4,
    H5,
    H6,
}

impl TryFrom<u8> for ATXHeadingLevel {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            2 => Ok(ATXHeadingLevel::H2),
            1 => Ok(ATXHeadingLevel::H1),
            3 => Ok(ATXHeadingLevel::H3),
            4 => Ok(ATXHeadingLevel::H4),
            5 => Ok(ATXHeadingLevel::H5),
            6 => Ok(ATXHeadingLevel::H6),
            _ => Err("value must be in range 1..=6"),
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct BlockQuote<'a> {
    pub children: Vec<Block<'a>>,
}

// #[derive(PartialEq, Debug)]
// pub enum SetextHeadingLevel {
//     H1,
//     H2,
// }

#[derive(PartialEq, Debug, Clone)]
pub enum ListType {
    OrderedDot(u32),
    OrdererParentheses(u32),
    UnorderedMinus,
    UnorderedPlus,
    UnorderedAsterisk,
}

impl ListType {
    pub fn same_variant(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct List<'a> {
    pub items: Vec<ListItem<'a>>,
    pub list_type: ListType,
    pub tight: bool,
}

#[derive(PartialEq, Debug, Clone)]
pub struct ListItem<'a> {
    pub children: Vec<Block<'a>>,
    pub padding: usize,
}

#[derive(PartialEq, Debug, Clone)]
pub struct LinkReference<'a> {
    pub label: &'a str,
    pub destination: &'a str,
    pub title: Option<&'a str>,
}

#[derive(PartialEq, Debug, Clone)]
pub enum FenceType {
    Backtick,
    Tilda,
}

#[derive(PartialEq, Debug, Clone)]
pub struct FencedCode<'a> {
    pub content: InlineContent<'a>,
    pub language: &'a str,
    pub ident: usize,
    pub fence_type: FenceType,
    pub fence_occ: usize,
}

#[derive(PartialEq, Debug, Clone)]
pub struct IndentedCode<'a> {
    pub content: InlineContent<'a>,
}
