fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    if let Err(err) = winresource::WindowsResource::new()
        .set_icon("assets/ico.ico")
        .compile()
    {
        println!("cargo:warning=failed to embed Windows icon resource: {err}");
    }
}
