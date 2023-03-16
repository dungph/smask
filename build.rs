fn main() {
    std::process::Command::new("touch")
        .arg("data.db")
        .output()
        .unwrap();
    println!("cargo:rerun-if-changed=migrations");
    println!("cargo:rerun-if-changed=data.db");
}
