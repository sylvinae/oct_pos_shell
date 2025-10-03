// Core API Commands for Pharmacy POS
#[tauri::command]
fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
fn get_platform() -> String {
    std::env::consts::OS.to_string()
}

// Hardware API Commands - Ready for implementation
#[tauri::command]
fn open_cash_drawer() -> Result<String, String> {
    // TODO: Implement actual hardware integration
    Ok("Cash drawer opened successfully".to_string())
}

#[tauri::command]
fn print_receipt(receipt_data: serde_json::Value) -> Result<String, String> {
    // TODO: Implement actual printer integration
    Ok("Receipt printed successfully".to_string())
}

// UI API Commands
#[tauri::command]
async fn show_alert(title: String, message: String, app: tauri::AppHandle) -> Result<String, String> {
    use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
    
    app.dialog()
        .message(&message)
        .title(&title)
        .kind(MessageDialogKind::Info)
        .buttons(MessageDialogButtons::Ok)
        .show(|_result| {
            // Dialog closed callback
        });
    
    Ok(format!("Alert shown: {} - {}", title, message))
}


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_app_version,
            get_platform,
            open_cash_drawer,
            print_receipt,
            show_alert
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
