/// Test binary detection
///
/// Run with: cargo run --example test_binary_detection
use stacy::executor::binary::detect_stata_binary;

fn main() {
    println!("Testing Stata binary detection...\n");

    // Test 1: Auto-detection (no CLI override)
    println!("1. Auto-detection (no CLI override):");
    println!("   Precedence: $STATA_BINARY > ~/.config/stacy/config.toml > auto-detect");
    match detect_stata_binary(None) {
        Ok(binary) => println!("   ✅ Found: {}", binary),
        Err(e) => println!("   ❌ Error: {}", e),
    }

    // Test 2: Environment variable (if set)
    println!("\n2. Environment variable $STATA_BINARY:");
    match std::env::var("STATA_BINARY") {
        Ok(env_val) => {
            println!("   Set to: {}", env_val);
            match detect_stata_binary(None) {
                Ok(binary) => println!("   ✅ Used: {}", binary),
                Err(e) => println!("   ❌ Error: {}", e),
            }
        }
        Err(_) => println!("   Not set (will use user config or auto-detection)"),
    }

    // Test 3: User config
    println!("\n3. User config (~/.config/stacy/config.toml):");
    match stacy::project::user_config::load_user_config() {
        Ok(Some(config)) => {
            if let Some(binary) = config.stata_binary {
                println!("   Configured: {}", binary);
            } else {
                println!("   Not configured (will use auto-detection)");
            }
        }
        Ok(None) => println!("   Config file not found (will use auto-detection)"),
        Err(e) => println!("   ❌ Error loading config: {}", e),
    }

    // Test 4: CLI flag override (testing with fake path)
    println!("\n4. CLI flag --engine (testing with fake path):");
    match detect_stata_binary(Some("/fake/stata-mp")) {
        Ok(binary) => println!("   ✅ Found: {}", binary),
        Err(e) => println!("   ❌ Expected error: {}", e),
    }

    // Test 5: CLI flag with real path
    println!("\n5. CLI flag --engine (testing with real path):");
    let real_path = "/Applications/StataNow/StataMP.app/Contents/MacOS/stata-mp";
    match detect_stata_binary(Some(real_path)) {
        Ok(binary) => println!("   ✅ Found: {}", binary),
        Err(e) => println!("   ❌ Error: {}", e),
    }
}
