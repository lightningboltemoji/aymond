mod basic;
mod no_sort_key;
mod no_table;
mod numeric_keys;

#[test]
fn compile() {
    let t = trybuild::TestCases::new();
    t.compile_fail("src/shouldnt_compile/*.rs");
}
