use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

/// Load HTML email templates from the given directory.
/// Each `.html` file becomes a template accessible by its filename.
pub async fn load_templates(template_dir: &str) -> HashMap<String, String> {
    let mut templates = HashMap::new();
    let dir = Path::new(template_dir);

    tracing::info!("Loading email templates from: {}", template_dir);

    if let Ok(mut entries) = fs::read_dir(dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("html") {
                if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                    match fs::read_to_string(&path).await {
                        Ok(content) => {
                            templates.insert(file_name.to_string(), content);
                            tracing::info!("Loaded template: {}", file_name);
                        }
                        Err(e) => {
                            tracing::error!("Failed to read template file '{}': {}", path.display(), e);
                        }
                    }
                }
            }
        }
    } else {
        tracing::error!("Templates directory '{}' not found or inaccessible (absolute path: {:?})", template_dir, dir.canonicalize().unwrap_or(dir.to_path_buf()));
    }

    templates
}
