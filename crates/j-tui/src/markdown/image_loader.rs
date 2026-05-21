use image::DynamicImage;

/// Expand `~/` prefix to the home directory.
fn expand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{}/{}", home, rest);
        }
        if let Ok(userprofile) = std::env::var("USERPROFILE") {
            return format!("{}\\{}", userprofile, rest);
        }
    }
    path.to_string()
}

/// 加载图片（支持 http/https URL、file:// URI 和本地路径）
pub fn load_image(source: &str) -> Result<DynamicImage, String> {
    if source.starts_with("http://") || source.starts_with("https://") {
        let bytes = reqwest::blocking::get(source)
            .map_err(|e| e.to_string())?
            .bytes()
            .map_err(|e| e.to_string())?;
        image::load_from_memory(&bytes).map_err(|e| e.to_string())
    } else {
        // 处理 file:// 协议：提取实际文件路径
        let path = if let Some(stripped) = source.strip_prefix("file://") {
            stripped.to_string()
        } else {
            expand_tilde(source)
        };
        image::open(&path).map_err(|e| e.to_string())
    }
}
