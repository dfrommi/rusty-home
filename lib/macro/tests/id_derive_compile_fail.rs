#[test]
fn unit_struct_without_fields_is_rejected() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/id_struct_unit_fail.rs");
}
