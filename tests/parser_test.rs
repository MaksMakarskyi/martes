use std::env;

use martes::{html::renderer::render_markdown, markdown::parser::parse};
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
fn parser_spec_tests() {
    let cases = load_spec();
    let mut failures: Vec<String> = Vec::new();

    for c in cases.iter() {
        let res = match parse(&c.markdown) {
            Ok(doc) => render_markdown(doc),
            Err(err) => {
                failures.push(format!(
                    "SPEC_TEST (example={}, section=\"{}\"):\n{:?}\n\nexpected: {:?}\n     got: {}",
                    c.example, c.section, c.markdown, c.html, err
                ));
                continue;
            }
        };

        if res != c.html {
            failures.push(format!(
                "SPEC_TEST (example={}, section=\"{}\"):\n{:?}\n\nexpected: {:?}\n     got: {:?}",
                c.example, c.section, c.markdown, c.html, res
            ));
        }
    }

    if !failures.is_empty() {
        let total = cases.len();
        let passed = total - failures.len();

        if env::args().any(|a| a == "--no-capture") {
            panic!(
                "{passed}/{total} passed, failed cases:\n\n{}\n\n",
                failures.join("\n\n")
            )
        } else {
            panic!("{passed}/{total} passed — run with \"--no-capture\" flag for details")
        }
    }
}
