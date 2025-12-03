use crate::some_or_return;
use crate::text::textbox::TextCoord;

#[derive(Clone, Debug)]
pub struct EditDetail {
    pub content: String,
    pub end: TextCoord,
}

#[derive(Clone, Debug)]
pub struct EditOp {
    pub caret: TextCoord,
    pub delete: Option<EditDetail>,
    pub insert: Option<EditDetail>,
}

pub struct EditHistory {
    max_history: usize,
    history: Vec<EditOp>,
    history_ptr: usize,
}

impl EditHistory {
    pub fn new(max_history: usize) -> Self {
        EditHistory {
            history: Vec::new(),
            history_ptr: 0,
            max_history,
        }
    }

    pub fn set_max_history(&mut self, max_history: usize) {
        self.max_history = max_history;
    }
    
    pub fn get_max_history(&self) -> usize {
        self.max_history
    }

    pub fn record_input(
        &mut self,
        caret: TextCoord,
        delete_detail: Option<EditDetail>,
        insert_detail: Option<EditDetail>,
    ) {
        if self.max_history == 0 || (delete_detail.is_none() && insert_detail.is_none()) {
            return;
        }
        if !self.merge_input(caret, &delete_detail, &insert_detail) {
            self.push_op(EditOp {
                caret,
                delete: delete_detail,
                insert: insert_detail,
            });
        }
    }

    pub fn undo(&mut self) -> Option<EditOp> {
        if self.history_ptr == 0 {
            return None;
        }
        let prev_op = unsafe { self.history.get_unchecked(self.history_ptr - 1) };
        self.history_ptr -= 1;
        Some(prev_op.clone())
    }

    pub fn redo(&mut self) -> Option<EditOp> {
        if self.history.is_empty() || self.history_ptr >= self.history.len() {
            return None;
        }
        let next_op = unsafe { self.history.get_unchecked(self.history_ptr) };
        self.history_ptr += 1;
        Some(next_op.clone())
    }

    fn merge_input(
        &mut self,
        caret: TextCoord,
        delete_detail: &Option<EditDetail>,
        insert_detail: &Option<EditDetail>,
    ) -> bool {
        if self.history_ptr == 0 || self.history_ptr != self.history.len() {
            return false;
        }
        let last_op = unsafe { self.history.get_unchecked_mut(self.history_ptr - 1) };
        match (
            delete_detail,
            insert_detail,
            &mut last_op.delete,
            &mut last_op.insert,
        ) {
            (None, Some(insert_detail), None, Some(last_insert)) => {
                if last_insert.end == caret
                    && !Self::starts_with_whitespace(&insert_detail.content)
                {
                    last_insert.content.push_str(&insert_detail.content);
                    last_insert.end = insert_detail.end;
                    true
                } else {
                    false
                }
            }
            (Some(delete_detail), None, Some(last_delete), None) => {
                if delete_detail.end == last_op.caret {
                    last_delete.content = format!("{}{}", delete_detail.content, last_delete.content);
                    last_op.caret = caret;
                    true
                } else {
                    false
                }
            }
            _ => false,
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

    fn starts_with_whitespace(content: &str) -> bool {
        let first = some_or_return!(content.chars().next(), false);
        ['\t', '\r', '\n', ' '].contains(&first)
    }

}
