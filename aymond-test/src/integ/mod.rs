mod basic;
mod batch_get;
mod binary_keys;
mod delete_item;
mod no_sort_key;
mod no_table;
mod numeric_keys;
mod option_attribute;
mod scan;
mod set_attribute;

#[test]
fn compile() {
    let t = trybuild::TestCases::new();
    t.compile_fail("src/shouldnt_compile/*.rs");
}
