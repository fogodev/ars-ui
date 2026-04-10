use ars_core::BindableValue;

fn assert_bindable_value<T: BindableValue>() {}

fn main() {
    assert_bindable_value::<String>();
    assert_bindable_value::<bool>();
    assert_bindable_value::<u32>();
    assert_bindable_value::<Vec<String>>();
}
