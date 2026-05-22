//! 图片渲染 pass 模块
//!
//! 根据图片标记处理各状态的图片（Ready/Failed/Loading/Pending），并在 Pending 时启动异步加载线程。

use super::selection::get_line_at;
use crate::command::chat::app::ChatApp;
use crate::command::chat::ui::title_bar;
use crate::markdown::image_cache::ImageState;
use crate::markdown::image_loader::load_image;
use crate::util::safe_lock;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use ratatui_image::{Resize, StatefulImage};

/// 图片渲染 pass：根据图片标记处理各状态的图片（Ready/Failed/Loading/Pending），
/// 并在 Pending 时启动异步加载线程。
pub(crate) fn render_image_pass(
    f: &mut ratatui::Frame,
    inner: Rect,
    img_markers: Vec<(usize, u16, String)>,
    start: usize,
    visible_height: u16,
    bubble_max_width: usize,
    app: &mut ChatApp,
) {
    let cached = match app.ui.msg_lines_cache.as_ref() {
        Some(c) => c,
        None => return,
    };
    let has_picker = safe_lock(&app.ui.image_cache, "draw_messages::image_cache_picker")
        .picker
        .is_some();
    let img_pad = 3u16; // 与气泡 pad_left_w 一致
    let img_render_w = (bubble_max_width as u16).saturating_sub(img_pad * 2);
    let history_total = cached.history_line_count;

    for (i, height, url) in img_markers {
        let line_idx = start + i;
        let y = inner.y + i as u16;
        let remaining_h = visible_height.saturating_sub(i as u16);
        let bubble_w = bubble_max_width as u16;

        // 计算实际可用的占位行数：从标记行往下数连续的空行/占位行
        let mut actual_h = 1u16;
        for next_offset in 1..height as usize {
            let next_idx = line_idx + next_offset;
            if next_idx >= cached.total_line_count {
                break;
            }
            let next_line = match get_line_at(cached, next_idx, history_total) {
                Some(l) => l,
                None => break,
            };
            let is_placeholder = next_line.spans.is_empty()
                || next_line
                    .spans
                    .iter()
                    .all(|s| s.content.replace('│', "").trim().is_empty());
            if is_placeholder {
                actual_h += 1;
            } else {
                break;
            }
        }
        let render_h = actual_h.min(height).min(remaining_h);

        if remaining_h < render_h {
            continue;
        }

        let img_x = inner.x + img_pad;
        let img_area = Rect::new(img_x, y, img_render_w, render_h);

        if !has_picker {
            let max_url_w = (bubble_w as usize).saturating_sub(12);
            let display_url = title_bar::truncate_str(&url, max_url_w);
            let fallback = Paragraph::new(Line::from(Span::styled(
                format!("  [Image: {}]", display_url),
                Style::default()
                    .fg(Color::Cyan)
                    .bg(app.ui.theme.bubble_ai)
                    .add_modifier(Modifier::UNDERLINED),
            )));
            f.render_widget(fallback, Rect::new(inner.x, y, bubble_w, 1));
            continue;
        }

        let mut cache = safe_lock(&app.ui.image_cache, "draw_chat_ui::image_cache");
        match cache.images.get_mut(&url) {
            Some(ImageState::Ready(protocol)) => {
                let widget = StatefulImage::default().resize(Resize::Scale(None));
                f.render_stateful_widget(widget, img_area, protocol);
            }
            Some(ImageState::Failed(err)) => {
                let max_err_w = (bubble_w as usize).saturating_sub(24);
                let display_err = title_bar::truncate_str(err, max_err_w);
                let err_line = Paragraph::new(Line::from(Span::styled(
                    format!("  [Image load failed: {}]", display_err),
                    Style::default().fg(Color::Red).bg(app.ui.theme.bubble_ai),
                )));
                f.render_widget(err_line, Rect::new(inner.x, y, bubble_w, 1));
            }
            Some(ImageState::Loading) => {
                let max_url_w = (bubble_w as usize).saturating_sub(21);
                let display_url = title_bar::truncate_str(&url, max_url_w);
                let loading = Paragraph::new(Line::from(Span::styled(
                    format!("  Loading image: {}...", display_url),
                    Style::default()
                        .fg(Color::DarkGray)
                        .bg(app.ui.theme.bubble_ai),
                )));
                f.render_widget(loading, Rect::new(inner.x, y, bubble_w, 1));
            }
            Some(ImageState::Pending) | None => {
                let max_url_w = (bubble_w as usize).saturating_sub(21);
                let display_url = title_bar::truncate_str(&url, max_url_w);
                let loading = Paragraph::new(Line::from(Span::styled(
                    format!("  Loading image: {}...", display_url),
                    Style::default()
                        .fg(Color::DarkGray)
                        .bg(app.ui.theme.bubble_ai),
                )));
                f.render_widget(loading, Rect::new(inner.x, y, bubble_w, 1));
                cache.images.insert(url.clone(), ImageState::Loading);
                let cache_clone = std::sync::Arc::clone(&app.ui.image_cache);
                let url_owned = url.clone();
                std::thread::spawn(move || match load_image(&url_owned) {
                    Ok(dyn_img) => {
                        let mut c = safe_lock(&cache_clone, "image_load::cache_ready");
                        if let Some(ref picker) = c.picker {
                            let protocol: ratatui_image::protocol::StatefulProtocol =
                                picker.new_resize_protocol(dyn_img);
                            c.images.insert(url_owned, ImageState::Ready(protocol));
                        }
                    }
                    Err(e) => {
                        safe_lock(&cache_clone, "image_load::cache_failed")
                            .images
                            .insert(url_owned, ImageState::Failed(e));
                    }
                });
            }
        }
    }
}
