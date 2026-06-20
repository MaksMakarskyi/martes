#[derive(PartialEq, Debug, Clone)]
pub enum Inline<'a> {
    CodeSpan(&'a str),
    Emphasis(Emphasis<'a>),
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
pub struct Emphasis<'a> {
    pub text: &'a str,
    pub emphasis_type: EmphasisType,
}

#[derive(PartialEq, Debug, Clone)]
pub struct Link<'a> {
    pub text: &'a str,
    pub url: &'a str,
    pub title: Option<&'a str>,
}

#[derive(PartialEq, Debug, Clone)]
pub struct Image<'a> {
    pub src: &'a str,
    pub alt: &'a str,
    pub title: Option<&'a str>,
}
