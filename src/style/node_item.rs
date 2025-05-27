use crate as deft;
use crate::style::YogaNode;
use deft_macros::mrc_object;
use yoga::{Align, Context, Direction, Display, Edge, FlexDirection, Justify, MeasureMode, Node, NodeRef, Overflow, PositionType, Size, StyleUnit, Wrap};

pub struct MeasureParams {
    pub width: f32,
    pub width_mode: MeasureMode,
    pub height: f32,
    pub height_mode: MeasureMode,
}

#[mrc_object]
struct CustomMeasureFn {
    callback: Box<dyn FnMut(MeasureParams) -> Size>,
}

extern "C" fn custom_measure_shadow(
    node_ref: NodeRef,
    width: f32,
    _width_mode: MeasureMode,
    height: f32,
    _height_mode: MeasureMode,
) -> Size {
    if let Some(ctx) = Node::get_context_mut(&node_ref) {
        if let Some(node_item) = ctx.downcast_mut::<NodeItem>() {
            node_item.calculate_shadow_layout(width, height, Direction::LTR);
            let sn = node_item._shadow_yn.as_ref().unwrap();
            return yoga::Size {
                width: sn.get_layout_width(),
                height: sn.get_layout_height()
            };
        }
    }
    unreachable!()
}

extern "C" fn custom_measure_fn(
    node_ref: NodeRef,
    width: f32,
    width_mode: MeasureMode,
    height: f32,
    height_mode: MeasureMode,
) -> Size {
    if let Some(ctx) = Node::get_context_mut(&node_ref) {
        if let Some(func) = ctx.downcast_mut::<CustomMeasureFn>() {
            let params = MeasureParams {
                width,
                width_mode,
                height,
                height_mode,
            };
            return (func.callback)(params);
        }
    }
    unreachable!()
}


#[mrc_object]
pub struct NodeItem {
    pub _yn: YogaNode,
    _shadow_yn: Option<YogaNode>,
    pub padding_top: StyleUnit,
    pub padding_bottom: StyleUnit,
    pub padding_left: StyleUnit,
    pub padding_right: StyleUnit,
    pub justify_content: Justify,
    pub flex_direction: FlexDirection,
    pub align_content: Align,
    pub align_items: Align,
    pub flex_wrap: Wrap,
    pub column_gap: f32,
    pub row_gap: f32,
    pub position_type: PositionType,
    pub display: Display,
    pub width: StyleUnit,
    pub height: StyleUnit,
    pub max_width: StyleUnit,
    pub max_height: StyleUnit,
    pub min_width: StyleUnit,
    pub min_height: StyleUnit,
    pub margin_top: StyleUnit,
    pub margin_right: StyleUnit,
    pub margin_bottom: StyleUnit,
    pub margin_left: StyleUnit,
    pub flex_basis: StyleUnit,
    pub top: StyleUnit,
    pub right: StyleUnit,
    pub bottom: StyleUnit,
    pub left: StyleUnit,
    pub overflow: Overflow,
    pub flex: f32,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub align_self: Align,
    pub direction: Direction,
    pub border_top: f32,
    pub border_right: f32,
    pub border_bottom: f32,
    pub border_left: f32,
    pub measure_fn: Option<CustomMeasureFn>,
    pub children: Vec<NodeItem>,
}

impl NodeItem {
    pub fn new() -> Self {
        let mut std_node = YogaNode::new();
        NodeItemData {
            _yn: YogaNode::new(),
            _shadow_yn: None,
            padding_top: std_node.get_style_padding_top(),
            padding_bottom: std_node.get_style_padding_bottom(),
            // padding_top: StyleUnit::UndefinedValue,
            // padding_bottom: StyleUnit::UndefinedValue,
            padding_left: std_node.get_style_padding_left(),
            padding_right: std_node.get_style_padding_right(),
            justify_content: std_node.get_justify_content(),
            flex_direction: std_node.get_flex_direction(),
            align_content: std_node.get_align_content(),
            align_items: std_node.get_align_items(),
            flex_wrap: std_node.get_flex_wrap(),
            column_gap: std_node.get_column_gap(),
            row_gap: std_node.get_row_gap(),
            position_type: PositionType::Static,
            display: Display::Flex,
            width: std_node.get_style_width(),
            height: std_node.get_style_height(),
            max_width: std_node.get_style_max_width(),
            max_height: std_node.get_style_max_height(),
            min_width: std_node.get_style_min_width(),
            min_height: std_node.get_style_min_height(),
            margin_top: std_node.get_style_margin_top(),
            margin_right:  std_node.get_style_margin_right(),
            margin_bottom: std_node.get_style_margin_bottom(),
            margin_left: std_node.get_style_margin_left(),
            flex_basis: std_node.get_flex_basis(),
            top: std_node.get_style_position_top(),
            right: std_node.get_style_position_right(),
            bottom: std_node.get_style_position_bottom(),
            left: std_node.get_style_position_left(),
            overflow: Overflow::Hidden,
            flex: std_node.get_flex(),
            flex_grow: f32::NAN,
            flex_shrink: f32::NAN,
            align_self: std_node.get_align_self(),
            direction: std_node.get_style_direction(),
            border_top: 0.0,
            border_right: 0.0,
            border_bottom: 0.0,
            border_left: 0.0,
            measure_fn: None,
            children: vec![],
        }
        .to_ref()
    }

    pub fn has_shadow(&self) -> bool {
        match self.overflow {
            Overflow::Visible => false,
            Overflow::Hidden => false,
            Overflow::Scroll => true,
        }
    }

    pub fn calculate_layout(
        &mut self,
        available_width: f32,
        available_height: f32,
        direction: Direction,
    ) {
        self.build_yoga_node(false);
        self._yn
            .calculate_layout(available_width, available_height, direction);
    }

    pub fn calculate_shadow_layout(
        &mut self,
        available_width: f32,
        available_height: f32,
        direction: Direction,
    ) {
        self.build_yoga_node(true);
        if let Some(sn) = &mut self._shadow_yn {
            sn.calculate_layout(available_width, available_height, direction);
        }
    }

    fn build_yoga_node(&mut self, is_shadow_root: bool) {
        let visit_children = is_shadow_root || !self.has_shadow();
        if visit_children {
            for c in &mut self.children {
                c.build_yoga_node(false);
            }
        }

        let mut n = YogaNode::new();
        let mut s = YogaNode::new();
        n.set_position_type(self.position_type);
        n.set_display(self.display);
        n.set_width(self.width);
        n.set_height(self.height);
        n.set_max_width(self.max_width);
        n.set_max_height(self.max_height);
        n.set_min_width(self.min_width);
        n.set_min_height(self.min_height);
        n.set_margin(Edge::Top, self.margin_top);
        n.set_margin(Edge::Bottom, self.margin_bottom);
        n.set_margin(Edge::Left, self.margin_left);
        n.set_margin(Edge::Right, self.margin_right);
        n.set_flex_basis(self.flex_basis);
        n.set_position(Edge::Top, self.top);
        n.set_position(Edge::Bottom, self.bottom);
        n.set_position(Edge::Left, self.left);
        n.set_position(Edge::Right, self.right);
        n.set_overflow(self.overflow);
        n.set_flex(self.flex);
        n.set_flex_grow(self.flex_grow);
        n.set_flex_shrink(self.flex_shrink);
        n.set_align_self(self.align_self);
        n.set_border(Edge::Top, self.border_top);
        n.set_border(Edge::Right, self.border_right);
        n.set_border(Edge::Bottom, self.border_bottom);
        n.set_border(Edge::Left, self.border_left);
        if self.has_shadow() {
            n.set_context(Some(Context::new(self.clone())));
            n.set_measure_func(Some(custom_measure_shadow));
        } else {
            if let Some(measure_func) = self.measure_fn.clone() {
                n.set_context(Some(Context::new(measure_func)));
                n.set_measure_func(Some(custom_measure_fn));
            }
        }

        let container = if self.has_shadow() { &mut s } else { &mut n };
        container.set_padding(Edge::Top, self.padding_top);
        container.set_padding(Edge::Bottom, self.padding_bottom);
        container.set_padding(Edge::Left, self.padding_left);
        container.set_padding(Edge::Right, self.padding_right);
        container.set_justify_content(self.justify_content);
        container.set_align_items(self.align_items);
        container.set_align_content(self.align_content);
        container.set_flex_direction(self.flex_direction);
        container.set_align_items(self.align_items);
        container.set_flex_wrap(self.flex_wrap);
        container.set_column_gap(self.column_gap);
        container.set_row_gap(self.row_gap);
        container.set_direction(self.direction);

        if visit_children {
            let mut idx = 0;
            for c in &mut self.children {
                container.insert_child(&mut c._yn, idx);
                idx += 1;
            }
        }
        if is_shadow_root {
            self._shadow_yn = if self.has_shadow() { Some(s) } else { None }
        } else {
            self._yn = n;
        }
    }

    pub fn set_measure_func<C: 'static, F: FnMut(&mut C, MeasureParams) -> Size + 'static>(&mut self, mut context: C, mut measure_func: F) {
        let func = CustomMeasureFnData {
            callback: Box::new(move |params| {
                measure_func(&mut context, params)
            }),
        }.to_ref();
        self.measure_fn = Some(func);
    }
    
}

#[cfg(test)]
mod tests {
    use ordered_float::OrderedFloat;
    use yoga::{Direction, Overflow, StyleUnit};
    use crate::style::node_item::NodeItem;

    #[test]
    fn test_layout() {
        let mut root = NodeItem::new();
        let mut child1 = NodeItem::new();
        let mut child2 = NodeItem::new();
        child1.width = StyleUnit::Percent(OrderedFloat(50.0));
        child1.height = StyleUnit::Point(OrderedFloat(10.0));
        child2.width = StyleUnit::Percent(OrderedFloat(50.0));
        child2.height = StyleUnit::Point(OrderedFloat(20.0));
        root.calculate_layout(100.0, 100.0, Direction::LTR);
        assert_eq!(root._yn.get_layout_width(), 100.0);
        assert_eq!(root._yn.get_layout_height(), 100.0);
    }

    #[test]
    fn test_overflow() {
        let mut root = NodeItem::new();
        let mut child1 = NodeItem::new();
        let mut child2 = NodeItem::new();
        root.overflow = Overflow::Scroll;
        child1.width = StyleUnit::Percent(OrderedFloat(50.0));
        child1.height = StyleUnit::Point(OrderedFloat(1000.0));
        child2.width = StyleUnit::Percent(OrderedFloat(50.0));
        child2.height = StyleUnit::Point(OrderedFloat(20.0));
        root.calculate_layout(100.0, 100.0, Direction::LTR);
        root.calculate_shadow_layout(100.0, 100.0, Direction::LTR);
        let sn = root._shadow_yn.as_ref().unwrap();
        assert_eq!(sn.get_layout_width(), 100.0);
        assert_eq!(sn.get_layout_height(), 100.0);
        // assert_eq!(root._yn.get_layout_height(), 20.0);
    }


}
