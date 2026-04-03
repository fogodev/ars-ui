//! ars-ui workspace task runner.

mod manifest;

fn main() {
    let cwd = std::env::current_dir().expect("cannot read current directory");
    let root = match manifest::SpecRoot::discover(&cwd) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };
    println!("Spec root: {}", root.path.display());
    println!("Components: {}", root.manifest.components.len());
}
