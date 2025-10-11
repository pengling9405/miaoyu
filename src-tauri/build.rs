fn main() {
    // 在构建时加载 .env 文件
    dotenvy::dotenv().ok();

    // 将环境变量传递给编译器，这样 option_env!() 才能读取到
    if let Ok(app_id) = std::env::var("VOLCENGINE_APP_ID") {
        println!("cargo:rustc-env=VOLCENGINE_APP_ID={}", app_id);
    }

    if let Ok(token) = std::env::var("VOLCENGINE_ACCESS_TOKEN") {
        println!("cargo:rustc-env=VOLCENGINE_ACCESS_TOKEN={}", token);
    }

    if let Ok(key) = std::env::var("DEEPSEEK_API_KEY") {
        println!("cargo:rustc-env=DEEPSEEK_API_KEY={}", key);
    }

    tauri_build::build()
}
