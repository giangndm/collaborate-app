#[test]
fn derive_ui_contract() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/derive_ok.rs");
    t.pass("tests/ui/container_fields_ok.rs");
    t.compile_fail("tests/ui/duplicate_id.rs");
    t.compile_fail("tests/ui/tuple_struct.rs");
    t.compile_fail("tests/ui/rename_conflict.rs");
    t.compile_fail("tests/ui/with_attr_rejected.rs");
    t.compile_fail("tests/ui/vec_missing_id.rs");
}
