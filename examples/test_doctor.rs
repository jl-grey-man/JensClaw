use sandy::config::Config;
use sandy::tools::doctor::DoctorTool;
use sandy::tools::Tool;

#[tokio::main]
async fn main() {
    println!("=== Testing Doctor Command ===\n");

    // Load actual config
    let config = Config::load()
        .expect("Failed to load config");

    println!("Loaded config:");
    println!("  - agents_file: {}", config.agents_file);
    println!("  - data_dir: {}", config.data_dir);
    println!();

    // Create doctor tool
    let doctor = DoctorTool::new(&config, vec!["bash".into(), "read_file".into(), "web_search".into()]);

    // Run diagnostics
    println!("Running diagnostics...\n");
    let result = doctor.execute(serde_json::json!({})).await;

    if result.is_error {
        println!("❌ Doctor command failed:");
        println!("{}", result.content);
    } else {
        println!("{}", result.content);
    }

    println!("\n=== Doctor Test Complete ===");
}
