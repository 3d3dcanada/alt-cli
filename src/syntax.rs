//! Local syntax extraction; no model and no project code execution.
use anyhow::Result;
use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize)]
pub struct Chunk {
    pub name: String,
    pub kind: String,
    pub start_line: usize,
    pub end_line: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Syntax {
    pub parser: Option<String>,
    pub contains_parse_errors: bool,
    pub chunks: Vec<Chunk>,
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Dependency {
    pub kind: String,
    pub start_line: usize,
    pub statement: String,
    pub module: Option<String>,
}
pub fn chunks(path: &str, body: &str) -> Result<Syntax> {
    let extension = std::path::Path::new(path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let language: tree_sitter::Language = match extension {
        "rs" => tree_sitter_rust::LANGUAGE.into(),
        "py" => tree_sitter_python::LANGUAGE.into(),
        "js" | "jsx" | "mjs" | "cjs" => tree_sitter_javascript::LANGUAGE.into(),
        _ => {
            return Ok(Syntax {
                parser: None,
                contains_parse_errors: false,
                chunks: vec![],
                dependencies: vec![],
            });
        }
    };
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&language)?;
    let tree = parser
        .parse(body, None)
        .ok_or_else(|| anyhow::anyhow!("Syntax parser did not finish"))?;
    let mut result = Syntax {
        parser: Some(format!("tree-sitter {extension}")),
        contains_parse_errors: tree.root_node().has_error(),
        chunks: vec![],
        dependencies: vec![],
    };
    fn visit(n: tree_sitter::Node, body: &str, out: &mut Vec<Chunk>, depth: usize) {
        if depth > 128 || out.len() >= 64 {
            return;
        }
        if matches!(
            n.kind(),
            "function_item"
                | "struct_item"
                | "enum_item"
                | "trait_item"
                | "impl_item"
                | "function_definition"
                | "class_definition"
                | "function_declaration"
                | "class_declaration"
                | "method_definition"
                | "lexical_declaration"
        ) && n.end_byte().saturating_sub(n.start_byte()) <= 16 * 1024
        {
            let name = n
                .child_by_field_name("name")
                .or_else(|| n.child_by_field_name("type"))
                .and_then(|n| n.utf8_text(body.as_bytes()).ok())
                .unwrap_or(n.kind());
            out.push(Chunk {
                name: name.into(),
                kind: n.kind().into(),
                start_line: n.start_position().row + 1,
                end_line: n.end_position().row + 1,
                start_byte: n.start_byte(),
                end_byte: n.end_byte(),
            });
        }
        let mut cursor = n.walk();
        for child in n.named_children(&mut cursor) {
            visit(child, body, out, depth + 1);
        }
    }
    visit(tree.root_node(), body, &mut result.chunks, 0);
    fn imports(n: tree_sitter::Node, body: &str, out: &mut Vec<Dependency>, depth: usize) {
        if depth > 128 || out.len() >= 64 {
            return;
        }
        if matches!(
            n.kind(),
            "import_statement" | "import_from_statement" | "use_declaration"
        ) {
            let module = n
                .child_by_field_name("source")
                .or_else(|| n.child_by_field_name("module_name"))
                .and_then(|s| s.utf8_text(body.as_bytes()).ok())
                .map(|s| s.trim_matches(['\'', '"']).to_owned());
            out.push(Dependency {
                kind: n.kind().into(),
                start_line: n.start_position().row + 1,
                statement: crate::project::bounded(n.utf8_text(body.as_bytes()).unwrap_or(""), 300),
                module,
            });
        }
        let mut cursor = n.walk();
        for child in n.named_children(&mut cursor) {
            imports(child, body, out, depth + 1);
        }
    }
    imports(tree.root_node(), body, &mut result.dependencies, 0);
    Ok(result)
}
