fn main() {
    // 在构建时加载 .env 文件
    dotenvy::dotenv().ok();

    if let Ok(key) = std::env::var("DEEPSEEK_API_KEY") {
        println!("cargo:rustc-env=DEEPSEEK_API_KEY={}", key);
    }

    tauri_build::build()
}
