use std::env;

fn main() {
    // 常规：把环境变量注入
    dotenvy::dotenv().ok();
    if let Ok(key) = env::var("DEEPSEEK_API_KEY") {
        println!("cargo:rustc-env=DEEPSEEK_API_KEY={}", key);
    }
    println!("cargo:rerun-if-changed=build.rs");
    tauri_build::build();
}
