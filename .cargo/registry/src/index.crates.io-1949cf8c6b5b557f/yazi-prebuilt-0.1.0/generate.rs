use std::fs;

use anyhow::Result;
use syntect::{dumps::dump_to_file, parsing::{SyntaxDefinition, SyntaxSetBuilder}};
use walkdir::WalkDir;

pub fn main() -> Result<()> {
	let mut builder = SyntaxSetBuilder::new();
	builder.add_plain_text_syntax();

	let dir =
		WalkDir::new("./syntaxes").follow_links(true).sort_by(|a, b| a.file_name().cmp(b.file_name()));

	for entry in dir {
		let entry = entry?;
		let path = entry.path();

		if path.extension().map_or(false, |e| e == "sublime-syntax") {
			let syn = SyntaxDefinition::load_from_str(
				&fs::read_to_string(path)?,
				true,
				path.file_stem().and_then(|x| x.to_str()),
			);

			match syn {
				Ok(syn) => builder.add(syn),
				Err(e) => println!("Failed to load syntax {:?}: {}", path, e),
			}
		}
	}

	dump_to_file(&builder.build(), "./built/syntaxes").unwrap();
	Ok(())
}
