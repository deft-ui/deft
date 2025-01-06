pub mod event;
pub mod js_interop;
pub mod element;
pub mod event_handling;
pub mod style;
pub mod obj_ref;
pub mod js_type;

#[macro_export]
macro_rules! ok_or_return {
    ($expr:expr) => {
        if let Ok(v) = $expr {
            v
        } else {
            return;
        }
    }
}

#[macro_export]
macro_rules! some_or_return {
    ($expr:expr) => {
        if let Some(v) = $expr {
            v
        } else {
            return;
        }
    };
    ($expr:expr, $default: expr) => {
        if let Some(v) = $expr {
            v
        } else {
            return $default;
        }
    }
}

#[macro_export]
macro_rules! some_or_continue {
    ($expr:expr) => {
        if let Some(v) = $expr {
            v
        } else {
            continue;
        }
    };
}