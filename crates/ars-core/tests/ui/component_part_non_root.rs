use ars_core::ComponentPart;

#[derive(ComponentPart)]
#[scope = "dialog"]
enum Part {
    Trigger,
    Root,
}

fn main() {}
