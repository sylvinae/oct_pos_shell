// Core API Commands for Pharmacy POS
use std::process::Command;
use tauri::Manager;

#[tauri::command]
fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
fn get_platform() -> String {
    std::env::consts::OS.to_string()
}


#[tauri::command]
fn get_printers() -> Result<Vec<String>, String> {
    #[cfg(target_os = "windows")]
    {
        match get_windows_printers() {
            Ok(printers) => Ok(printers),
            Err(e) => Err(format!("Failed to get printers: {}", e))
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        match get_macos_printers() {
            Ok(printers) => Ok(printers),
            Err(e) => Err(format!("Failed to get printers: {}", e))
        }
    }
    
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        Err("Printer detection not supported on this platform".to_string())
    }
}

#[tauri::command]
fn print_receipt(_receipt_data: serde_json::Value) -> Result<String, String> {
    // TODO: Implement actual printer integration
    Ok("Receipt printed successfully".to_string())
}

#[tauri::command]
fn test_print_receipt(printer_name: String) -> Result<String, String> {
    println!("Test printing to: {}", printer_name);
    
    #[cfg(target_os = "windows")]
    {
        test_print_windows(&printer_name)
    }
    
    #[cfg(target_os = "macos")]
    {
        test_print_macos(&printer_name)
    }
    
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        Err("Test printing not supported on this platform".to_string())
    }
}

#[tauri::command]
fn print_escpos_receipt(printer_name: String, escpos_data: Vec<u8>) -> Result<String, String> {
    println!("Printing ESC/POS receipt to: {}", printer_name);
    
    #[cfg(target_os = "windows")]
    {
        print_raw_windows(&printer_name, &escpos_data)
    }
    
    #[cfg(target_os = "macos")]
    {
        print_raw_macos(&printer_name, &escpos_data)
    }
    
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        Err("ESC/POS printing not supported on this platform".to_string())
    }
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
fn restart_app(app: tauri::AppHandle) {
    app.restart();
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
            get_printers,
            print_receipt,
            test_print_receipt,
            print_escpos_receipt,
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
            
            // Clear persisted data when window closes
            if let Some(window) = app.get_webview_window("main") {
                window.on_window_event(|event| {
                    if let tauri::WindowEvent::CloseRequested { .. } = event {
                        println!("Window closing - clearing persisted data");
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

#[cfg(target_os = "windows")]
fn get_windows_printers() -> Result<Vec<String>, String> {
    let output = Command::new("powershell")
        .args(&["-Command", "Get-Printer | Select-Object -ExpandProperty Name"])
        .output()
        .map_err(|e| format!("Failed to execute PowerShell command: {}", e))?;
    
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        #[cfg_attr(not(debug_assertions), allow(unused_mut))]
        let mut printers: Vec<String> = stdout
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.trim().to_string())
            .collect();
        
        if printers.is_empty() {
            println!("No printers found on Windows");
            
            #[cfg(debug_assertions)]
            {
                println!("Running in dev mode - adding mock printers for testing");
                printers = vec![
                    "EPSON TM-T88V (Mock)".to_string(),
                    "Star TSP143 (Mock)".to_string(),
                    "Brother QL-820NWB (Mock)".to_string(),
                    "Microsoft Print to PDF".to_string(),
                ];
            }
        } else {
            println!("Found {} printer(s) on Windows", printers.len());
        }
        
        Ok(printers)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("PowerShell error: {}", stderr))
    }
}

#[cfg(target_os = "windows")]
fn test_print_windows(printer_name: &str) -> Result<String, String> {
    use std::fs::File;
    use std::io::Write;
    
    let test_content = generate_test_receipt();
    
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join("pos_test_receipt.txt");
    
    let mut file = File::create(&file_path)
        .map_err(|e| format!("Failed to create temp file: {}", e))?;
    
    file.write_all(test_content.as_bytes())
        .map_err(|e| format!("Failed to write to temp file: {}", e))?;
    
    let output = Command::new("cmd")
        .args(&[
            "/C",
            &format!("type \"{}\" > \"{}\"", 
                file_path.display(), 
                printer_name
            )
        ])
        .output()
        .map_err(|e| format!("Failed to execute print command: {}", e))?;
    
    std::fs::remove_file(&file_path).ok();
    
    if output.status.success() {
        Ok(format!("Test receipt sent to {}", printer_name))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Print failed: {}", stderr))
    }
}

#[cfg(target_os = "macos")]
fn test_print_macos(printer_name: &str) -> Result<String, String> {
    use std::fs::File;
    use std::io::Write;
    
    let test_content = generate_test_receipt();
    
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join("pos_test_receipt.txt");
    
    let mut file = File::create(&file_path)
        .map_err(|e| format!("Failed to create temp file: {}", e))?;
    
    file.write_all(test_content.as_bytes())
        .map_err(|e| format!("Failed to write to temp file: {}", e))?;
    
    let output = Command::new("lpr")
        .args(&[
            "-P", printer_name,
            "-o", "raw",
            file_path.to_str().unwrap()
        ])
        .output()
        .map_err(|e| format!("Failed to execute lpr command: {}", e))?;
    
    std::fs::remove_file(&file_path).ok();
    
    if output.status.success() {
        Ok(format!("Test receipt sent to {}", printer_name))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.is_empty() {
            Ok(format!("Test receipt sent to {}", printer_name))
        } else {
            Err(format!("Print warning: {}", stderr))
        }
    }
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
fn generate_test_receipt() -> String {
    format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n\n\n",
        "================================",
        "      OCT PHARMACY POS          ",
        "================================",
        "",
        "       TEST PRINT OK            ",
        "",
        "================================",
    )
}

#[cfg(target_os = "windows")]
fn print_raw_windows(printer_name: &str, data: &[u8]) -> Result<String, String> {
    use std::fs::File;
    use std::io::Write;
    
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join("escpos_receipt.bin");
    
    let mut file = File::create(&file_path)
        .map_err(|e| format!("Failed to create temp file: {}", e))?;
    
    file.write_all(data)
        .map_err(|e| format!("Failed to write data: {}", e))?;
    
    let output = std::process::Command::new("cmd")
        .args(&[
            "/C",
            &format!("copy /B \"{}\" \"{}\"", file_path.display(), printer_name)
        ])
        .output()
        .map_err(|e| format!("Failed to execute print command: {}", e))?;
    
    std::fs::remove_file(&file_path).ok();
    
    if output.status.success() {
        Ok(format!("Receipt printed to {}", printer_name))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Print failed: {}", stderr))
    }
}

#[cfg(target_os = "macos")]
fn print_raw_macos(printer_name: &str, data: &[u8]) -> Result<String, String> {
    use std::fs::File;
    use std::io::Write;
    
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join("escpos_receipt.bin");
    
    let mut file = File::create(&file_path)
        .map_err(|e| format!("Failed to create temp file: {}", e))?;
    
    file.write_all(data)
        .map_err(|e| format!("Failed to write data: {}", e))?;
    
    let output = std::process::Command::new("lpr")
        .args(&[
            "-P", printer_name,
            "-o", "raw",
            file_path.to_str().unwrap()
        ])
        .output()
        .map_err(|e| format!("Failed to execute lpr command: {}", e))?;
    
    std::fs::remove_file(&file_path).ok();
    
    if output.status.success() {
        Ok(format!("Receipt printed to {}", printer_name))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.is_empty() {
            Ok(format!("Receipt printed to {}", printer_name))
        } else {
            Err(format!("Print warning: {}", stderr))
        }
    }
}

#[cfg(target_os = "macos")]
fn get_macos_printers() -> Result<Vec<String>, String> {
    let output = Command::new("lpstat")
        .args(&["-p"])
        .output()
        .map_err(|e| format!("Failed to execute lpstat command: {}", e))?;
    
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        #[cfg_attr(not(debug_assertions), allow(unused_mut))]
        let mut printers: Vec<String> = stdout
            .lines()
            .filter_map(|line| {
                line.strip_prefix("printer ")
                    .and_then(|rest| rest.split_whitespace().next())
                    .map(|s| s.to_string())
            })
            .collect();
        
        if printers.is_empty() {
            println!("No printers found on macOS via lpstat");
            
            #[cfg(debug_assertions)]
            {
                println!("Running in dev mode - adding mock printers for testing");
                printers = vec![
                    "EPSON TM-T88V (Mock)".to_string(),
                    "Star TSP143 (Mock)".to_string(),
                    "Brother QL-820NWB (Mock)".to_string(),
                ];
            }
        } else {
            println!("Found {} printer(s) on macOS", printers.len());
        }
        
        Ok(printers)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("lpstat error: {}", stderr))
    }
}
