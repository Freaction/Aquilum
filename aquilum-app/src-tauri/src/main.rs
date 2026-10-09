#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--mcp-stdio") {
        std::process::exit(aquilum_app_lib::run_mcp_stdio_bridge());
    }
    if args.iter().any(|a| a == "--mcp-stdio-server") {
        std::process::exit(aquilum_app_lib::run_mcp_stdio_server());
    }
    aquilum_app_lib::run()
}
