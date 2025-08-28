// Rope data structure for efficient text editing
// Based on Xi editor's rope implementation
// https://xi-editor.io/docs/rope_science_01.html

use std::rc::Rc;
use std::cmp::min;

// Leaf size - Xi uses 1024, we'll use 512 for smaller PDFs
const LEAF_SIZE: usize = 512;
// Min leaf size after splitting
const MIN_LEAF: usize = LEAF_SIZE / 2;

#[derive(Clone, Debug)]
pub enum Node {
    Leaf(String),
    Internal {
        left: Rc<Node>,
        right: Rc<Node>,
        len: usize,  // Total length of text in this subtree
        line_breaks: usize,  // Number of \n in this subtree
    },
}

impl Node {
    pub fn len(&self) -> usize {
        match self {
            Node::Leaf(s) => s.len(),
            Node::Internal { len, .. } => *len,
        }
    }

    pub fn line_breaks(&self) -> usize {
        match self {
            Node::Leaf(s) => s.chars().filter(|&c| c == '\n').count(),
            Node::Internal { line_breaks, .. } => *line_breaks,
        }
    }

    pub fn height(&self) -> usize {
        match self {
            Node::Leaf(_) => 0,
            Node::Internal { left, .. } => left.height() + 1,
        }
    }

    pub fn is_balanced(&self) -> bool {
        match self {
            Node::Leaf(_) => true,
            Node::Internal { left, right, .. } => {
                let left_height = left.height();
                let right_height = right.height();
                let diff = if left_height > right_height {
                    left_height - right_height
                } else {
                    right_height - left_height
                };
                diff <= 1 && left.is_balanced() && right.is_balanced()
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Rope {
    root: Rc<Node>,
}

impl Rope {
    /// Create a new rope from a string
    pub fn from_str(s: &str) -> Self {
        if s.is_empty() {
            return Rope {
                root: Rc::new(Node::Leaf(String::new())),
            };
        }

        // Split into leaves
        let mut nodes = Vec::new();
        let mut start = 0;
        
        while start < s.len() {
            let end = min(start + LEAF_SIZE, s.len());
            // Try to break at a line boundary if possible
            let mut actual_end = end;
            if end < s.len() {
                // Look for a newline near the split point
                if let Some(newline_pos) = s[start..end].rfind('\n') {
                    if newline_pos > MIN_LEAF {
                        actual_end = start + newline_pos + 1;
                    }
                }
            }
            
            nodes.push(Rc::new(Node::Leaf(s[start..actual_end].to_string())));
            start = actual_end;
        }

        // Build tree from leaves
        while nodes.len() > 1 {
            let mut new_nodes = Vec::new();
            let mut i = 0;
            
            while i < nodes.len() {
                if i + 1 < nodes.len() {
                    new_nodes.push(Rc::new(Node::Internal {
                        len: nodes[i].len() + nodes[i + 1].len(),
                        line_breaks: nodes[i].line_breaks() + nodes[i + 1].line_breaks(),
                        left: nodes[i].clone(),
                        right: nodes[i + 1].clone(),
                    }));
                    i += 2;
                } else {
                    new_nodes.push(nodes[i].clone());
                    i += 1;
                }
            }
            
            nodes = new_nodes;
        }

        Rope {
            root: nodes.into_iter().next().unwrap(),
        }
    }

    /// Get the total length of the rope
    pub fn len(&self) -> usize {
        self.root.len()
    }

    /// Get the number of lines
    pub fn line_count(&self) -> usize {
        self.root.line_breaks() + 1
    }

    /// Get a character at a specific offset
    pub fn char_at(&self, offset: usize) -> Option<char> {
        if offset >= self.len() {
            return None;
        }

        fn char_at_node(node: &Node, offset: usize) -> Option<char> {
            match node {
                Node::Leaf(s) => s.chars().nth(offset),
                Node::Internal { left, right, .. } => {
                    let left_len = left.len();
                    if offset < left_len {
                        char_at_node(left, offset)
                    } else {
                        char_at_node(right, offset - left_len)
                    }
                }
            }
        }

        char_at_node(&self.root, offset)
    }

    /// Get a substring from the rope
    pub fn substring(&self, start: usize, end: usize) -> String {
        if start >= end || start >= self.len() {
            return String::new();
        }

        let actual_end = min(end, self.len());
        let mut result = String::new();
        
        fn collect_substring(node: &Node, start: usize, end: usize, result: &mut String) {
            match node {
                Node::Leaf(s) => {
                    let chars: Vec<char> = s.chars().collect();
                    let actual_end = min(end, chars.len());
                    for i in start..actual_end {
                        result.push(chars[i]);
                    }
                }
                Node::Internal { left, right, .. } => {
                    let left_len = left.len();
                    
                    if start < left_len {
                        collect_substring(left, start, min(end, left_len), result);
                    }
                    
                    if end > left_len {
                        let right_start = if start > left_len { start - left_len } else { 0 };
                        let right_end = end - left_len;
                        collect_substring(right, right_start, right_end, result);
                    }
                }
            }
        }

        collect_substring(&self.root, start, actual_end, &mut result);
        result
    }

    /// Insert text at a specific offset
    pub fn insert(&mut self, offset: usize, text: &str) -> Self {
        if text.is_empty() {
            return self.clone();
        }

        let before = self.substring(0, offset);
        let after = self.substring(offset, self.len());
        
        let new_text = format!("{}{}{}", before, text, after);
        Rope::from_str(&new_text)
    }

    /// Delete a range of text
    pub fn delete(&mut self, start: usize, end: usize) -> Self {
        if start >= end || start >= self.len() {
            return self.clone();
        }

        let before = self.substring(0, start);
        let after = self.substring(end, self.len());
        
        let new_text = format!("{}{}", before, after);
        Rope::from_str(&new_text)
    }

    /// Convert the rope to a string
    pub fn to_string(&self) -> String {
        self.substring(0, self.len())
    }

    /// Find the offset of the nth line
    pub fn line_offset(&self, line_index: usize) -> Option<usize> {
        if line_index > self.line_count() {
            return None;
        }

        fn find_line_offset(node: &Node, target_line: usize, current_offset: usize) -> Option<usize> {
            if target_line == 0 {
                return Some(current_offset);
            }

            match node {
                Node::Leaf(s) => {
                    let mut line_count = 0;
                    for (i, ch) in s.chars().enumerate() {
                        if ch == '\n' {
                            line_count += 1;
                            if line_count == target_line {
                                return Some(current_offset + i + 1);
                            }
                        }
                    }
                    None
                }
                Node::Internal { left, right, .. } => {
                    let left_lines = left.line_breaks();
                    
                    if target_line <= left_lines {
                        find_line_offset(left, target_line, current_offset)
                    } else {
                        find_line_offset(right, target_line - left_lines, current_offset + left.len())
                    }
                }
            }
        }

        find_line_offset(&self.root, line_index, 0)
    }

    /// Get a specific line
    pub fn line(&self, line_index: usize) -> Option<String> {
        let start = self.line_offset(line_index)?;
        
        // Find the end of the line
        let mut end = start;
        while end < self.len() {
            if self.char_at(end) == Some('\n') {
                break;
            }
            end += 1;
        }
        
        Some(self.substring(start, end))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rope_creation() {
        let text = "Hello, world!\nThis is a test.";
        let rope = Rope::from_str(text);
        assert_eq!(rope.len(), text.len());
        assert_eq!(rope.to_string(), text);
    }

    #[test]
    fn test_char_at() {
        let rope = Rope::from_str("Hello");
        assert_eq!(rope.char_at(0), Some('H'));
        assert_eq!(rope.char_at(4), Some('o'));
        assert_eq!(rope.char_at(5), None);
    }

    #[test]
    fn test_line_operations() {
        let text = "Line 1\nLine 2\nLine 3";
        let rope = Rope::from_str(text);
        assert_eq!(rope.line_count(), 3);
        assert_eq!(rope.line(0), Some("Line 1".to_string()));
        assert_eq!(rope.line(1), Some("Line 2".to_string()));
        assert_eq!(rope.line(2), Some("Line 3".to_string()));
    }

    #[test]
    fn test_insert() {
        let mut rope = Rope::from_str("Hello world");
        rope = rope.insert(5, ",");
        assert_eq!(rope.to_string(), "Hello, world");
    }

    #[test]
    fn test_delete() {
        let mut rope = Rope::from_str("Hello, world!");
        rope = rope.delete(5, 7);
        assert_eq!(rope.to_string(), "Helloworld!");
    }
}