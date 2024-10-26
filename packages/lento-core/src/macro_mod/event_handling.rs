#[macro_export]
macro_rules! match_event {
    ($actual_type: expr, $event: expr, $expected_type: expr, $target: expr, $func: ident) => {
        if $actual_type == $expected_type {
            $target.$func();
        }
    };
    ($actual_type: expr, $event: expr, $expected_type: expr, $detail_type: ty, $target: expr, $func: ident) => {
        if $actual_type == $expected_type {
            if let Some(detail) = $event.detail.downcast_ref::<$detail_type>() {
                $target.$func(detail);
                return;
            }
        }
    };
}

#[macro_export]
macro_rules! match_event_type {
    ($event: expr, $detail_type: ty, $target: expr, $func: ident) => {
        if let Some(detail) = $event.detail.raw().downcast_ref::<$detail_type>() {
            $target.$func(detail);
            return;
        }
    };
}