//! Regenerates `src/bindings.ts` without launching the GUI.
//!
//! `create_specta()` writes the bindings as a side effect under
//! `debug_assertions`, but it is normally only reached from `run()`, which
//! starts the whole app. Booting the app to refresh a type file leaves the
//! export racing against process teardown and can truncate the output.
//!
//! Run with: cargo test -p trickle --test export_bindings
//!
//! The export path is relative to the crate directory, which is where cargo
//! sets the working directory for integration tests.

#[test]
fn export_typescript_bindings() {
    let _ = trickle_lib::create_specta();

    let path = std::path::Path::new("../src/bindings.ts");
    assert!(path.exists(), "bindings.ts was not written");

    let content = std::fs::read_to_string(path).unwrap();
    // Guard against a truncated write: the file must end with a complete
    // statement, not mid-type.
    assert!(
        content.trim_end().ends_with('}') || content.trim_end().ends_with(';'),
        "bindings.ts looks truncated"
    );
}
