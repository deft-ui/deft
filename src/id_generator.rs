pub struct IdGenerator {
    next_id: u32,
}

impl IdGenerator {
    pub fn new() -> Self {
        Self { next_id: 1 }
    }

    pub fn generate_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}
