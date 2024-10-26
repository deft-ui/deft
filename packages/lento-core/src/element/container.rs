use crate::element::{ElementBackend, ElementRef};

pub struct Container {
    dirty: bool,
    children: Vec<ElementRef>,
    element: ElementRef,
}

impl Container {

}

impl ElementBackend for Container {
    fn create(element: ElementRef) -> Self {
        Self {
            dirty: false,
            children: Vec::new(),
            element,
        }
    }

    fn get_name(&self) -> &str {
        "Container"
    }

    fn add_child_view(&mut self, mut child: ElementRef, position: Option<u32>) {
        if let Some(p) = child.get_parent() {
            panic!("child({}) has parent({}) already", child.get_id(), p.get_id());
        }
        let ele = &mut self.element;
        let pos = {
            let layout = &mut ele.layout;
            let pos = position.unwrap_or_else(|| layout.child_count());
            layout.insert_child(&mut child.layout, pos);
            pos
        };
        child.set_parent(Some(ele.clone()));
        self.children.insert(pos as usize, child);

        ele.with_window(|win| {
            win.mark_dirty(true);
        });
    }

    fn remove_child_view(&mut self, position: u32) {
        let mut c = self.children.remove(position as usize);
        c.set_parent(None);
        let mut ele = self.element.clone();
        let layout = &mut ele.layout;
        layout.remove_child(&mut c.layout);
        ele.mark_dirty(true);
    }

    fn get_children(&self) -> Vec<ElementRef> {
        self.children.clone()
    }

}

