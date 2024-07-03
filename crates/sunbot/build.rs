
fn main() {
    println!("Building!");

    // let dst = path::Path::new(&env::var("OUT_DIR").expect("OUT_DIR not set")).join("built.rs");
    // write_built_file_with_opts(
    //     #[cfg(any(feature = "cargo-lock", feature = "git2"))]
    //     Some(
    //         env::var("CARGO_MANIFEST_DIR")
    //             .expect("CARGO_MANIFEST_DIR")
    //             .as_ref(),
    //     ),
    //     &dst,
    // )?;
    // Ok(())

    built::write_built_file().expect("Failed to acquire build-time information");
}
