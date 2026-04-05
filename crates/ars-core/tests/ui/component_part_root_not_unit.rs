use ars_core::ComponentPart;

#[derive(ComponentPart)]
#[scope = "dialog"]
enum Part {
    Root(String),
    Trigger,
}

fn main() {}
