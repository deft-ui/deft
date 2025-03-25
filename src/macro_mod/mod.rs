pub mod element;
pub mod style;
pub mod js_type;
mod performance;

#[macro_export]
macro_rules! ok_or_return {
    ($expr:expr) => {
        if let Ok(v) = $expr {
            v
        } else {
            return;
        }
    };
    ($expr:expr, $default: expr) => {
        if let Ok(v) = $expr {
            v
        } else {
            return $default;
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

#[macro_export]
macro_rules! some_or_break {
    ($expr:expr) => {
        if let Some(v) = $expr {
            v
        } else {
            break;
        }
    };
}