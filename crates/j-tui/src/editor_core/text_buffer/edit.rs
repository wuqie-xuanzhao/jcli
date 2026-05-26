//! 文本编辑操作

use super::TextBuffer;

impl TextBuffer {
    // ========== 文本编辑 ==========

    /// 在当前光标位置插入字符
    pub fn insert_char(&mut self, ch: char) {
        let (row, col) = self.cursor;
        if let Some(line) = self.lines.get_mut(row) {
            let byte_offset = Self::byte_offset_of(line, col);
            line.insert(byte_offset, ch);
            self.cursor.1 = col + 1;
            self.modified = true;
        }
    }

    /// 在当前光标位置插入字符串
    pub fn insert_str(&mut self, s: &str) {
        if !s.contains('\n') {
            let (row, col) = self.cursor;
            if let Some(line) = self.lines.get_mut(row) {
                let byte_offset = Self::byte_offset_of(line, col);
                line.insert_str(byte_offset, s);
                self.cursor.1 = col + s.chars().count();
                self.modified = true;
            }
        } else {
            for ch in s.chars() {
                if ch == '\n' {
                    self.insert_newline();
                } else {
                    self.insert_char(ch);
                }
            }
        }
    }

    /// 在当前光标位置插入换行
    pub fn insert_newline(&mut self) {
        let (row, col) = self.cursor;
        if let Some(line) = self.lines.get(row) {
            let byte_offset = Self::byte_offset_of(line, col);
            let before = line[..byte_offset].to_string();
            let after = line[byte_offset..].to_string();

            self.lines[row] = before;
            self.lines.insert(row + 1, after);
            self.cursor = (row + 1, 0);
            self.modified = true;
        }
    }

    /// 删除光标位置的字符
    pub fn delete_char(&mut self) {
        let (row, col) = self.cursor;
        if let Some(line) = self.lines.get_mut(row) {
            let chars_count = line.chars().count();
            if col < chars_count {
                let byte_start = Self::byte_offset_of(line, col);
                let byte_end = Self::byte_offset_of(line, col + 1);
                line.drain(byte_start..byte_end);
                self.modified = true;
            } else if row < self.lines.len() - 1 {
                // 合并下一行
                let next_line = self.lines.remove(row + 1);
                self.lines[row].push_str(&next_line);
                self.modified = true;
            }
        }
    }

    /// 删除光标前的字符（退格）
    pub fn backspace(&mut self) {
        if self.cursor.1 > 0 {
            self.cursor.1 -= 1;
            self.delete_char();
        } else if self.cursor.0 > 0 {
            // 合并到上一行
            let current_line = self.lines.remove(self.cursor.0);
            self.cursor.0 -= 1;
            let prev_line_len = self.lines[self.cursor.0].chars().count();
            self.lines[self.cursor.0].push_str(&current_line);
            self.cursor.1 = prev_line_len;
            self.modified = true;
        }
    }

    /// 删除当前行
    pub fn delete_line(&mut self) {
        if self.lines.len() > 1 {
            self.lines.remove(self.cursor.0);
            if self.cursor.0 >= self.lines.len() {
                self.cursor.0 = self.lines.len() - 1;
            }
            self.cursor.1 = self.cursor.1.min(self.current_line_len());
        } else {
            self.lines[0].clear();
            self.cursor.1 = 0;
        }
        self.modified = true;
    }

    /// 删除从光标到行尾的内容
    pub fn delete_line_by_end(&mut self) {
        let (row, col) = self.cursor;
        if let Some(line) = self.lines.get_mut(row) {
            let byte_offset = Self::byte_offset_of(line, col);
            line.truncate(byte_offset);
            self.modified = true;
        }
    }

    /// 删除当前单词
    pub fn delete_word(&mut self) {
        let (row, col) = self.cursor;
        if let Some(line) = self.lines.get(row) {
            let mut end = col;

            // 跳过空白
            for ch in line.chars().skip(col) {
                if !ch.is_whitespace() {
                    break;
                }
                end += 1;
            }
            // 跳过单词字符
            for ch in line.chars().skip(end) {
                if ch.is_whitespace() {
                    break;
                }
                end += 1;
            }

            if end > col
                && let Some(line) = self.lines.get_mut(row)
            {
                let byte_start = Self::byte_offset_of(line, col);
                let byte_end = Self::byte_offset_of(line, end);
                line.drain(byte_start..byte_end);
                self.modified = true;
            }
        }
    }

    /// 在当前行下方插入新行
    pub fn insert_line_below(&mut self) {
        let row = self.cursor.0;
        self.lines.insert(row + 1, String::new());
        self.cursor = (row + 1, 0);
        self.modified = true;
    }

    /// 在当前行上方插入新行
    pub fn insert_line_above(&mut self) {
        let row = self.cursor.0;
        self.lines.insert(row, String::new());
        self.cursor = (row, 0);
        self.modified = true;
    }

    // ========== 批量操作 ==========

    /// 替换所有行（用于撤销/重做）
    pub fn replace_lines(&mut self, lines: Vec<String>) {
        self.lines = lines;
        // 确保至少有一行
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        // 确保光标位置有效
        self.cursor.0 = self.cursor.0.min(self.lines.len() - 1);
        self.cursor.1 = self.cursor.1.min(self.current_line_len());
        self.modified = true;
    }

    /// 获取快照（用于撤销）
    pub fn snapshot(&self) -> Vec<String> {
        self.lines.clone()
    }
}
