fn main() {
    println!("cargo:rerun-if-changed=../ui/index.html");
    println!("cargo:rerun-if-changed=../ui/styles.css");
    println!("cargo:rerun-if-changed=../ui/app.js");
    tauri_build::build()
}
