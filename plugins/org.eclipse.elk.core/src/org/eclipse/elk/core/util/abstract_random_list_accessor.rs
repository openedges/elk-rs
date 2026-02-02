pub trait AbstractRandomListAccessor<T> {
    fn list(&self) -> &Vec<T>;
    fn list_mut(&mut self) -> &mut Vec<T>;
    fn provide_default(&self) -> T;

    fn get_list_item(&mut self, index: usize) -> &mut T {
        self.ensure_list_size(index + 1);
        &mut self.list_mut()[index]
    }

    fn set_list_item(&mut self, index: usize, value: T) {
        if index < self.list().len() {
            self.list_mut()[index] = value;
        } else {
            self.ensure_list_size(index);
            self.list_mut().push(value);
        }
    }

    fn get_list_size(&self) -> usize {
        self.list().len()
    }

    fn clear_list(&mut self) {
        self.list_mut().clear();
    }

    fn ensure_list_size(&mut self, size: usize) {
        let current_len = self.list().len();
        if current_len >= size {
            return;
        }
        for _ in current_len..size {
            let value = self.provide_default();
            self.list_mut().push(value);
        }
    }
}
