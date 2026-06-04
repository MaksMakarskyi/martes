#[derive(PartialEq, Debug, Clone)]
pub enum Inline<'a> {
    CodeSpan(&'a str),
    Emphasis(&'a str, EmphasisType),
    Link(Link<'a>),
    Image(Image<'a>),
    Autolink(&'a str),
    HTML(&'a str),
    HardLineBreak,
    TextualContent(&'a str),
}

#[derive(PartialEq, Debug, Clone)]
pub enum EmphasisType {
    Common,
    Strong,
}

#[derive(PartialEq, Debug, Clone)]
pub struct Link<'a> {
    text: &'a str,
    url: &'a str,
}

#[derive(PartialEq, Debug, Clone)]
pub struct Image<'a> {
    url: &'a str,
    alt: &'a str,
    title: &'a str,
}
