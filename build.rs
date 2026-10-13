//! Embeds the Windows version resource (`app.rc`) so the executable carries
//! publisher/product metadata (Company name, copyright, file description).

fn main() {
    println!("cargo:rerun-if-changed=app.rc");

    #[cfg(target_os = "windows")]
    embed_resource::compile("app.rc", embed_resource::NONE)
        .manifest_optional()
        .unwrap();
}
