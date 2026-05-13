use ars_i18n::Translate;

#[derive(Translate)]
#[translate(fallback = "en")]
enum BadText {
    #[translate(en = "{} items")]
    Count { count: usize },
}

fn main() {}
