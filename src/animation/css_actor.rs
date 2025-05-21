use crate::animation::actor::AnimationActor;
use crate::animation::Animation;
use crate::element::ElementWeak;
use crate::ok_or_return;

pub struct CssAnimationActor {
    animation: Animation,
    element: ElementWeak,
}

impl CssAnimationActor {
    pub fn new(animation: Animation, element: ElementWeak) -> Self {
        Self { animation, element }
    }
}

impl AnimationActor for CssAnimationActor {
    fn apply_animation(&mut self, position: f32, _stop: &mut bool) {
        let mut el = ok_or_return!(self.element.upgrade());
        el.animation_style_props.clear();
        let styles = self.animation.get_frame(position);
        for st in styles {
            el.animation_style_props.insert(st.key().clone(), st);
        }
        el.mark_style_dirty();
    }

    fn stop(&mut self) {
        let mut el = ok_or_return!(self.element.upgrade());
        el.animation_style_props.clear();
        el.mark_style_dirty();
    }
}
