fn main() {
    #[cfg(windows)]
    winres::WindowsResource::new()
        .set_icon("assets/icon.ico")
        .set("ProductName", "Typing Simulator")
        .set("FileDescription", "Typing Simulator")
        .compile()
        .expect("failed to embed the Windows application icon");
}
