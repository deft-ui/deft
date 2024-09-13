use crate::string::StringUtils;

#[derive(PartialEq, Eq)]
pub enum EditOpType {
    Insert,
    Delete,
}

pub struct EditOp {
    pub caret: usize,
    pub op: EditOpType,
    pub content: String,
}

pub struct EditHistory {
    max_history: usize,
    history: Vec<EditOp>,
    history_ptr: usize,
}

impl EditHistory {
    pub fn new() -> Self {
        EditHistory {
            history: Vec::new(),
            history_ptr: 0,
            max_history: 20,
        }
    }

    pub fn record_input(&mut self, caret: usize, content: &str) {
        if !self.merge_input(caret, content) {
            self.push_op(EditOp {
                caret,
                op: EditOpType::Insert,
                content: content.to_string(),
            });
        }
    }

    pub fn record_delete(&mut self, caret: usize, content: &str) {
        if !self.merge_delete(caret, content) {
            self.push_op(EditOp {
                caret,
                op: EditOpType::Delete,
                content: content.to_string(),
            });
        }
    }

    pub fn undo(&mut self) -> Option<EditOp> {
        if self.history_ptr == 0 {
            return None;
        }
        let prev_op = unsafe { self.history.get_unchecked(self.history_ptr - 1) };
        self.history_ptr -= 1;
        let op = match prev_op.op {
            EditOpType::Insert => {
                EditOp {
                    caret: prev_op.caret,
                    op: EditOpType::Delete,
                    content: prev_op.content.to_string(),
                }
            }
            EditOpType::Delete => {
                EditOp {
                    caret: prev_op.caret,
                    op: EditOpType::Insert,
                    content: prev_op.content.to_string(),
                }
            }
        };
        Some(op)
    }

    fn merge_input(&mut self, op_caret: usize, content: &str) -> bool {
        if self.history_ptr == 0 || self.history_ptr != self.history.len() {
            return false;
        }
        let last_op = unsafe { self.history.get_unchecked_mut(self.history_ptr - 1) };
        if last_op.op == EditOpType::Insert && last_op.caret + last_op.content.chars_count() == op_caret {
            last_op.content.push_str(content);
            true
        } else {
            false
        }
    }

    fn merge_delete(&mut self, op_caret: usize, content: &str) -> bool {
        if self.history_ptr == 0 || self.history_ptr != self.history.len() {
            return false;
        }
        let last_op = unsafe { self.history.get_unchecked_mut(self.history_ptr - 1) };
        if last_op.op != EditOpType::Delete {
            return false;
        }
        if last_op.caret + last_op.content.chars_count() == op_caret {
            last_op.content.push_str(content);
            true
        } else if op_caret + content.chars_count() == last_op.caret {
            last_op.caret = op_caret;
            let mut content = content.to_string();
            content.push_str(&last_op.content);
            last_op.content = content;
            true
        } else {
            false
        }
    }

    fn push_op(&mut self, op: EditOp) {
        let expected_len = self.history_ptr;
        while self.history.len() > expected_len {
            self.history.pop().unwrap();
        }
        if self.history.len() >= self.max_history {
            self.history.remove(0);
            self.history_ptr -= 1;
        }
        self.history.push(op);
        self.history_ptr += 1;
    }
}