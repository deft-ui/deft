use crate::base::Rect;
use crate::paint::{ElementObjectData, LayerObjectData, RenderLayerKey, RenderObject};
use crate::{some_or_continue, some_or_return};

pub struct LayoutTree {
    pub root_render_object: Option<RenderObject>,
    pub layer_objects: Vec<LayerObjectData>,
}

impl LayoutTree {
    pub fn new() -> Self {
        Self {
            root_render_object: None,
            layer_objects: Vec::new(),
        }
    }

    pub fn sync_invalid_area(mut self, new_tree: &mut Self) {
        // self.scroll_invalid_area(new_tree);
        self.merge_invalid_area(new_tree);
        for new_layer in &mut new_tree.layer_objects {
            let layer = some_or_continue!(self.get_layer_object_by_key(&new_layer.key));
            new_layer.invalid_area = layer.invalid_area.clone();
            new_layer.surface_bounds = layer.surface_bounds.clone();
            new_layer.visible_bounds = layer.visible_bounds.clone();
        }
    }

    pub fn get_all_layer_keys(&self) -> Vec<RenderLayerKey> {
        self.layer_objects.iter().map(|l| l.key.clone()).collect()
    }

    /*
    pub fn scroll_invalid_area(&mut self, new_tree: &Self) {
        for layer in &mut self.layer_objects {
            if let Some(new_layer) = new_tree.get_layer_object_by_key(&layer.key) {
                let scroll_delta_x = new_layer.scroll_left - layer.scroll_left;
                let scroll_delta_y = new_layer.scroll_top - layer.scroll_top;
                if scroll_delta_x != 0.0 || scroll_delta_y != 0.0 {
                    layer.invalid_area.offset(-scroll_delta_x, -scroll_delta_y);
                    if scroll_delta_y > 0.0 {
                        layer.invalid_area.add_rect(Some(&skia_safe::Rect::new(0.0, layer.height - scroll_delta_y, layer.width, layer.height)));
                    } else if scroll_delta_y < 0.0 {
                        layer.invalid_area.add_rect(Some(&skia_safe::Rect::new(0.0, 0.0, layer.width, -scroll_delta_y)));
                    }

                    if scroll_delta_x > 0.0 {
                        layer.invalid_area.add_rect(Some(&skia_safe::Rect::new(layer.width - scroll_delta_x, 0.0, layer.width, layer.height)));
                    } else if scroll_delta_x < 0.0 {
                        layer.invalid_area.add_rect(Some(&skia_safe::Rect::new(0.0, 0.0, -scroll_delta_x, layer.height)));
                    }
                }
            }
        }
    }
     */

    pub fn merge_invalid_area(&mut self, new_tree: &Self) {
        let root = some_or_return!(&self.root_render_object).clone();
        self.merge_object_invalid_area(0, &root, new_tree);
    }

    fn merge_objects_invalid_area(
        &mut self,
        parent_layer_idx: usize,
        render_objects: &Vec<RenderObject>,
        new_tree: &Self,
    ) {
        for ro in render_objects {
            self.merge_object_invalid_area(parent_layer_idx, ro, new_tree);
        }
    }

    fn merge_object_invalid_area(
        &mut self,
        parent_layer_idx: usize,
        render_object: &RenderObject,
        new_tree: &Self,
    ) {
        match render_object {
            RenderObject::Normal(eo) => {
                self.merge_objects_invalid_area(parent_layer_idx, &eo.children, new_tree);
            }
            RenderObject::Layer(lo) => {
                let layer = &self.layer_objects[lo.layer_object_idx];
                let layer_key = layer.key.clone();

                if new_tree.get_layer_object_by_key(&layer_key).is_none() {
                    let layer_origin_pos = layer.origin_absolute_pos.clone();
                    let layer_width = layer.width;
                    let layer_height = layer.height;
                    let parent_layer = &mut self.layer_objects[parent_layer_idx];
                    let rect = Rect::new(
                        layer_origin_pos.0 - parent_layer.origin_absolute_pos.0,
                        layer_origin_pos.1 - parent_layer.origin_absolute_pos.1,
                        layer_width,
                        layer_height,
                    );
                    parent_layer
                        .invalid_area
                        .add_rect(Some(&rect.to_skia_rect()));
                }
                self.merge_objects_invalid_area(lo.layer_object_idx, &lo.objects, new_tree);
            }
        }
    }

    fn get_layer_object_by_key(&self, key: &RenderLayerKey) -> Option<&LayerObjectData> {
        for lo in &self.layer_objects {
            if &lo.key == key {
                return Some(lo);
            }
        }
        None
    }
}

impl Default for LayoutTree {
    fn default() -> Self {
        Self::new()
    }
}