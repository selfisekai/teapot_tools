use std::path::Path;

use teapot_tools::cloner::clone_dependencies;
use teapot_tools::deps_parser::parse_deps;

fn main() {
    let spec = parse_deps(Path::new("test_assets/chromium.DEPS")).unwrap();

    // println!("{:#?}", spec);
    clone_dependencies(&spec, Path::new("test_battlefield"));
}
