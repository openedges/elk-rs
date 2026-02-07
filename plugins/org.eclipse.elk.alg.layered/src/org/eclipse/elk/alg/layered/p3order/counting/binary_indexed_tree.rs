#[derive(Clone, Debug)]
pub struct BinaryIndexedTree {
    binary_sums: Vec<i32>,
    nums_per_index: Vec<i32>,
    size: i32,
    max_num: usize,
}

impl BinaryIndexedTree {
    pub fn new(max_num: usize) -> Self {
        BinaryIndexedTree {
            binary_sums: vec![0; max_num + 1],
            nums_per_index: vec![0; max_num],
            size: 0,
            max_num,
        }
    }

    pub fn add(&mut self, index: usize) {
        self.size += 1;
        if index >= self.nums_per_index.len() {
            return;
        }
        self.nums_per_index[index] += 1;
        let mut i = index + 1;
        while i < self.binary_sums.len() {
            self.binary_sums[i] += 1;
            let lowbit = i & i.wrapping_neg();
            i += lowbit;
        }
    }

    pub fn rank(&self, index: usize) -> i32 {
        let mut i = index.min(self.binary_sums.len().saturating_sub(1));
        let mut sum = 0;
        while i > 0 {
            sum += self.binary_sums[i];
            let lowbit = i & i.wrapping_neg();
            i -= lowbit;
        }
        sum
    }

    pub fn size(&self) -> i32 {
        self.size
    }

    pub fn remove_all(&mut self, index: usize) {
        if index >= self.nums_per_index.len() {
            return;
        }
        let num_entries = self.nums_per_index[index];
        if num_entries == 0 {
            return;
        }
        self.nums_per_index[index] = 0;
        self.size -= num_entries;
        let mut i = index + 1;
        while i < self.binary_sums.len() {
            self.binary_sums[i] -= num_entries;
            let lowbit = i & i.wrapping_neg();
            i += lowbit;
        }
    }

    pub fn clear(&mut self) {
        self.binary_sums = vec![0; self.max_num + 1];
        self.nums_per_index = vec![0; self.max_num];
        self.size = 0;
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }
}
