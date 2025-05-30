pub struct TodoItem {
    pub id: u32,
    pub title: String,
    pub completed: bool,
}

impl TodoItem {
    pub fn new(id: u32, title: String) -> Self {
        Self {
            id,
            title,
            completed: false,
        }
    }

    pub fn mark_completed(&mut self) {
        self.completed = true;
    }

    pub fn mark_incomplete(&mut self) {
        self.completed = false;
    }

    pub fn toggle(&mut self) {
        self.completed = !self.completed;
    }
}