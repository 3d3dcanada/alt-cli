use alt_cli::project::Project;
use std::path::Path;
fn main() {
 let root=Path::new("/tmp/alt-harness-audit/state");
 let cwd=root.join("project"); std::fs::create_dir_all(&cwd).unwrap();
 std::fs::write(cwd.join("Cargo.toml"),"[package]\nname=\"fixture\"\nversion=\"0.1.0\"\n").unwrap();
 std::fs::create_dir_all(cwd.join("src")).unwrap();
 std::fs::write(cwd.join("src/lib.rs"),"pub fn median(_values: &[i64]) -> Option<f64> { None }\n").unwrap();
 let mut p=Project::open(&root.join("data"),&cwd).unwrap();
 let original="Implement median(&[i64]) -> Option<f64>. Preserve the public API and do not change Cargo.toml.";
 p.start_task("conversation",original).unwrap();
 let memory=p.compact_memory("conversation",original,4096).unwrap();
 println!("FIRST_CONTEXT_INCLUDES_MEDIAN_SOURCE={}",memory.contains("pub fn median"));
 println!("FIRST_CONTEXT_INCLUDES_CARGO={}",memory.contains("[package]"));
 p.start_task("conversation","Correction: preserve the exact marker USER_SECOND_TURN_REQUIREMENT in all later work").unwrap();
 p.start_task("conversation","Continue").unwrap();
 let memory=p.compact_memory("conversation","Continue",4096).unwrap();
 println!("THIRD_CONTEXT_INCLUDES_ORIGINAL={}",memory.contains(original));
 println!("THIRD_CONTEXT_INCLUDES_SECOND_TURN_REQUIREMENT={}",memory.contains("USER_SECOND_TURN_REQUIREMENT"));
 println!("THIRD_CONTEXT={memory}");
}
