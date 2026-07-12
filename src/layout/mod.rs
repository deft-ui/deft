use std::cell::RefCell;
use yoga::{Direction, Layout};
use crate::number::DeNan;
use crate::state::{State, StateManager};
use crate::style::YogaNode;

thread_local! {
    static NODE_MGR: RefCell<StateManager> = RefCell::new(StateManager::new());
}

struct LayoutNodeInfo {
    _yn: YogaNode,
    _shadow_yn: Option<YogaNode>,
    shadow_layout_calculated: bool,
}

impl LayoutNodeInfo {
    fn new(layout_node: YogaNode) -> LayoutNodeInfo {
        Self {
            _yn: layout_node,
            _shadow_yn: None,
            shadow_layout_calculated: false,
        }
    }
}

#[derive(Clone)]
pub struct LayoutNode {
    parent: Option<Box<LayoutNode>>,
    state: State<LayoutNodeInfo>,
    children: Vec<LayoutNode>,
}


impl LayoutNode {
    pub fn new(layout_node: YogaNode) -> LayoutNode {
        let state = NODE_MGR.with_borrow_mut(|mgr| mgr.new_state(LayoutNodeInfo::new(layout_node)));
        Self { state, parent: None, children: Vec::new() }
    }

    pub fn update_yn(&self, yn: YogaNode) -> anyhow::Result<()> {
        let mut state = self.state.upgrade()?;
        state._yn = yn;
        Ok(())
    }

    pub fn update_shadow_yn(&self, yn: Option<YogaNode>) -> anyhow::Result<()> {
        let mut state = self.state.upgrade()?;
        state._shadow_yn = yn;
        state.shadow_layout_calculated = false;
        Ok(())
    }

    pub fn get_yn(&self) -> anyhow::Result<YogaNode> {
        let state = self.state.upgrade()?;
        Ok(state._yn.clone())
    }

    pub fn get_padding(&self) -> anyhow::Result<[f32; 4]> {
        let state = self.state.upgrade()?;
        Ok([
            state._yn.get_layout_padding_top().de_nan(0.0),
            state._yn.get_layout_padding_right().de_nan(0.0),
            state._yn.get_layout_padding_bottom().de_nan(0.0),
            state._yn.get_layout_padding_left().de_nan(0.0),
        ])
    }

    pub fn get_shadow_size(&self) -> anyhow::Result<Option<(f32, f32)>> {
        let state = self.state.upgrade()?;
        let r = state._shadow_yn.as_ref()
            .map(|n| (n.get_layout_width(), n.get_layout_width()));
        Ok(r)
    }

    pub fn get_border(&self) -> anyhow::Result<[f32; 4]> {
        let state = self.state.upgrade()?;
        let bl = state._yn.get_layout_border_left().de_nan(0.0);
        let br = state._yn.get_layout_border_right().de_nan(0.0);
        let bt = state._yn.get_layout_border_top().de_nan(0.0);
        let bb = state._yn.get_layout_border_bottom().de_nan(0.0);
        Ok([bt, br, bb, bl])
    }

    pub fn get_size(&self) -> anyhow::Result<[f32; 2]> {
        let state = self.state.upgrade()?;
        let width = state._yn.get_layout_width().de_nan(0.0);
        let height = state._yn.get_layout_height().de_nan(0.0);
        Ok([width, height])
    }

    pub fn get_layout(&self) -> anyhow::Result<Layout> {
        let state = self.state.upgrade()?;
        Ok(state._yn.get_layout())
    }

    pub fn calculate_layout(&self, available_width: f32, available_height: f32, direction: Direction) -> anyhow::Result<()> {
        let mut state = self.state.upgrade()?;
        state._yn.calculate_layout(available_width, available_height, direction);
        Ok(())
    }

    pub fn calculate_shadow_layout(&self, available_width: f32, available_height: f32, direction: Direction) -> anyhow::Result<()> {
        let mut state = self.state.upgrade()?;
        if let Some(yn) = &mut state._shadow_yn {
            yn.calculate_layout(available_width, available_height, direction);
            state.shadow_layout_calculated = true;
        }
        Ok(())
    }
    
    pub fn is_shadow_layout_calculated(&self) -> anyhow::Result<bool> {
        let state = self.state.upgrade()?;
        Ok(state.shadow_layout_calculated)
    }
}
