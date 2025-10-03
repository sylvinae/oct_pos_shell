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

#[tauri::command]
fn restart_app(app: tauri::AppHandle) -> Result<String, String> {
    app.restart();
    Ok("Restarting app...".to_string())
}


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            get_app_version,
            get_platform,
            open_cash_drawer,
            print_receipt,
            show_alert,
            restart_app
        ])
        .setup(|app| {
            #[cfg(desktop)]
            {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    loop {
                        let _ = check_for_updates(handle.clone()).await;
                        tokio::time::sleep(tokio::time::Duration::from_secs(600)).await;
                    }
                });
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(desktop)]
async fn check_for_updates(app: tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    use tauri_plugin_updater::UpdaterExt;
    use tauri::Emitter;
    
    if let Some(update) = app.updater()?.check().await? {
        let version = update.version.clone();
        let mut downloaded = 0;
        
        println!("Downloading update version {}...", version);
        
        update.download_and_install(
            |chunk_length, content_length| {
                downloaded += chunk_length;
                println!("Downloaded {}/{:?}", downloaded, content_length);
            },
            || {
                println!("Download finished, installing...");
            },
        ).await?;
        
        println!("Update {} installed successfully! Ready to restart.", version);
        
        let _ = app.emit("update-ready", version.to_string());
    }
    
    Ok(())
}
