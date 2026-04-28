use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

pub async fn load_templates() -> HashMap<String, String> {
    let mut templates = HashMap::new();
    let template_dir = Path::new("templates");
    
    if let Ok(abs_path) = std::env::current_dir() {
        tracing::info!("Searching for templates in: {}/templates", abs_path.display());
    }

    if let Ok(mut entries) = fs::read_dir(template_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("html") {
                if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                    if let Ok(content) = fs::read_to_string(&path).await {
                        templates.insert(file_name.to_string(), content);
                        tracing::info!("Loaded template: {}", file_name);
                    }
                }
            }
        }
    } else {
        tracing::warn!("Templates directory not found or inaccessible");
    }

    templates
}
