//! 视觉光标移动（考虑折行）

use super::TextBuffer;

/// 对单个逻辑行按显示宽度折行（与 crate::util::text::wrap_text 逻辑一致）
/// 返回折行后的视觉行列表
fn wrap_single_line(line: &str, max_width: usize) -> Vec<String> {
    use crate::util::text::char_width;
    let max_width = max_width.max(2);
    let mut result = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0;

    for ch in line.chars() {
        let ch_width = char_width(ch);
        if current_width + ch_width > max_width && !current_line.is_empty() {
            result.push(current_line.clone());
            current_line.clear();
            current_width = 0;
        }
        current_line.push(ch);
        current_width += ch_width;
    }
    if !current_line.is_empty() {
        result.push(current_line);
    }
    if result.is_empty() {
        result.push(String::new());
    }
    result
}

/// 根据视觉 X 偏移（显示宽度）反算逻辑列位置
/// visual_x: 目标位置在逻辑行中的显示宽度偏移（从 0 开始）
fn map_visual_x_to_logical_col(line: &str, visual_x: usize, _wrap_width: usize) -> usize {
    use crate::util::text::char_width;
    let mut col = 0;
    let mut acc_width = 0;
    for ch in line.chars() {
        let cw = char_width(ch);
        if acc_width + cw > visual_x {
            break;
        }
        acc_width += cw;
        col += 1;
    }
    col
}

impl TextBuffer {
    /// 向上移动光标一个视觉行（考虑视觉折行）
    /// wrap_width: 视觉折行宽度（显示宽度，中文字符占2宽度）
    pub fn move_cursor_visual_up(&mut self, wrap_width: usize) {
        use crate::util::text::char_width;
        let (row, col) = self.cursor;

        // 计算当前光标的视觉 X 偏移（显示宽度）
        let line = &self.lines[row];
        let visual_x: usize = line.chars().take(col).map(char_width).sum();

        // 当前逻辑行折行后的视觉行
        let wrapped = wrap_single_line(line, wrap_width);

        // 找当前光标所在视觉行
        let mut cumulative_width = 0usize;
        let mut current_visual_row = 0;
        for (vi, vl) in wrapped.iter().enumerate() {
            let vl_display_width: usize = vl.chars().map(char_width).sum();
            if cumulative_width + vl_display_width > visual_x {
                current_visual_row = vi;
                break;
            }
            cumulative_width += vl_display_width;
            current_visual_row = vi;
        }

        if current_visual_row > 0 {
            // 同一逻辑行内上移一个视觉行
            let target_visual_row = current_visual_row - 1;
            // 找目标视觉行的起始显示宽度偏移
            let mut target_start_width = 0usize;
            for seg in wrapped.iter().take(target_visual_row) {
                target_start_width += seg.chars().map(char_width).sum::<usize>();
            }
            // 目标视觉行的显示宽度
            let target_vl_width: usize = wrapped[target_visual_row]
                .chars()
                .map(char_width)
                .sum::<usize>();
            // 保持视觉 X 偏移不变（但不超出目标视觉行宽度）
            let target_visual_x = visual_x.min(target_start_width + target_vl_width);
            // 反算逻辑列位置
            let new_col = map_visual_x_to_logical_col(line, target_visual_x, wrap_width);
            self.cursor.1 = new_col;
        } else if row > 0 {
            // 跳到上一逻辑行的最后一个视觉行
            self.cursor.0 = row - 1;
            let prev_line = &self.lines[row - 1];
            let prev_wrapped = wrap_single_line(prev_line, wrap_width);
            let last_vis_row = prev_wrapped.len() - 1;
            // 找最后一行的起始显示宽度偏移
            let mut last_start_width = 0usize;
            for seg in prev_wrapped.iter().take(last_vis_row) {
                last_start_width += seg.chars().map(char_width).sum::<usize>();
            }
            let last_vl_width: usize = prev_wrapped[last_vis_row]
                .chars()
                .map(char_width)
                .sum::<usize>();
            let target_visual_x = visual_x.min(last_start_width + last_vl_width);
            let new_col = map_visual_x_to_logical_col(prev_line, target_visual_x, wrap_width);
            self.cursor.1 = new_col.min(prev_line.chars().count());
        }
    }

    /// 向下移动光标一个视觉行（考虑视觉折行）
    /// wrap_width: 视觉折行宽度（显示宽度，中文字符占2宽度）
    pub fn move_cursor_visual_down(&mut self, wrap_width: usize) {
        use crate::util::text::char_width;
        let (row, col) = self.cursor;

        // 计算当前光标的视觉 X 偏移（显示宽度）
        let line = &self.lines[row];
        let visual_x: usize = line.chars().take(col).map(char_width).sum();

        // 当前逻辑行折行后的视觉行
        let wrapped = wrap_single_line(line, wrap_width);
        let visual_row_count = wrapped.len();

        // 找当前光标所在视觉行
        let mut cumulative_width = 0usize;
        let mut current_visual_row = 0;
        for (vi, vl) in wrapped.iter().enumerate() {
            let vl_display_width: usize = vl.chars().map(char_width).sum();
            if cumulative_width + vl_display_width > visual_x {
                current_visual_row = vi;
                break;
            }
            cumulative_width += vl_display_width;
            current_visual_row = vi;
        }

        if current_visual_row < visual_row_count - 1 {
            // 同一逻辑行内下移一个视觉行
            let target_visual_row = current_visual_row + 1;
            // 找目标视觉行的起始显示宽度偏移
            let mut target_start_width = 0usize;
            for seg in wrapped.iter().take(target_visual_row) {
                target_start_width += seg.chars().map(char_width).sum::<usize>();
            }
            // 目标视觉行的显示宽度
            let target_vl_width: usize = wrapped[target_visual_row]
                .chars()
                .map(char_width)
                .sum::<usize>();
            // 保持视觉 X 偏移不变（但不超出目标视觉行宽度）
            let target_visual_x = visual_x.min(target_start_width + target_vl_width);
            // 反算逻辑列位置
            let new_col = map_visual_x_to_logical_col(line, target_visual_x, wrap_width);
            self.cursor.1 = new_col;
        } else if row < self.lines.len() - 1 {
            // 跳到下一逻辑行的第一个视觉行
            self.cursor.0 = row + 1;
            let next_line = &self.lines[row + 1];
            let next_wrapped = wrap_single_line(next_line, wrap_width);
            // 第一行的显示宽度
            let first_vl_width: usize = next_wrapped[0].chars().map(char_width).sum();
            let target_visual_x = visual_x.min(first_vl_width);
            let new_col = map_visual_x_to_logical_col(next_line, target_visual_x, wrap_width);
            self.cursor.1 = new_col.min(next_line.chars().count());
        }
    }

    /// 计算当前逻辑行的视觉行数（用于判断是否有视觉折行）
    pub fn visual_line_count(&self, row: usize, wrap_width: usize) -> usize {
        if let Some(line) = self.lines.get(row) {
            wrap_single_line(line, wrap_width).len()
        } else {
            1
        }
    }
}
