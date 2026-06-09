use martes::markdown::parse;
use serde::Deserialize;

#[derive(Deserialize)]
struct SpecCase {
    markdown: String,
    html: String,
    example: u32,
    section: String,
}

fn load_spec() -> Vec<SpecCase> {
    let json = include_str!("spec.json");
    serde_json::from_str(json).expect("spec.json failed to parse")
}

#[test]
fn tests() {
    let cases = load_spec();
    for c in cases.iter() {
        let res = parse(&c.markdown).unwrap().to_html();
        assert_eq!(
            res, c.html,
            "\n\nSPEC_TEST (example={}, section=\"{}\"):\n{:?}\n",
            c.example, c.section, c.markdown
        )
    }
}
