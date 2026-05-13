use ars_i18n::Translate;

#[derive(Translate)]
#[translate(fallback = "en")]
enum Text {
    #[translate(en = "{count} items")]
    Count { total: usize },
}

fn main() {}
