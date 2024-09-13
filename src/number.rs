pub trait DeNan {
    fn de_nan(self, default: Self) -> Self;

    fn nan_to_zero(self) -> Self;
}

impl DeNan for f32 {
    fn de_nan(self, default: Self) -> Self {
        if self.is_nan() { default } else { self }
    }

    fn nan_to_zero(self) -> Self {
        self.de_nan(0.0)
    }
}