use super::IThreeValueGrid;

const LSB_MASK: u64 = 0x01;
const TWO_LSBS_MASK: u64 = 0x03;
const EMPTY: u64 = 0x00;
const BLOCKED: u64 = 0x01;
const WEAKLY_BLOCKED: u64 = 0x02;
const REST_MASK: usize = 0x1f;
const RIGHT_SHIFT: usize = 5; // 2^5 = 32

#[derive(Clone, Debug)]
pub struct TwoBitGrid {
    grid: Vec<Vec<u64>>,
    x_size: usize,
    y_size: usize,
}

impl TwoBitGrid {
    pub fn new(width: usize, height: usize) -> Self {
        let words = if width == 0 { 0 } else { (width + 31) / 32 };
        TwoBitGrid {
            grid: vec![vec![0_u64; words]; height],
            x_size: width,
            y_size: height,
        }
    }

    fn check_bounds(&self, x: usize, y: usize) {
        if x >= self.x_size || y >= self.y_size {
            panic!(
                "Grid is only of size {}*{}. Requested point ({}, {}) is out of bounds.",
                self.x_size, self.y_size, x, y
            );
        }
    }

    fn retrieve(&self, x: usize, y: usize) -> u64 {
        self.check_bounds(x, y);
        let x_word = x >> RIGHT_SHIFT;
        let x_rest = x & REST_MASK;
        (self.grid[y][x_word] >> (x_rest << 1)) & TWO_LSBS_MASK
    }

    fn set_bits(&mut self, x: usize, y: usize, msb: bool, lsb: bool) {
        self.check_bounds(x, y);
        let x_word = x >> RIGHT_SHIFT;
        let x_rest = x & REST_MASK;
        let mut mask = LSB_MASK << (x_rest << 1);
        if lsb {
            self.grid[y][x_word] |= mask;
        } else {
            self.grid[y][x_word] &= !mask;
        }
        mask <<= 1;
        if msb {
            self.grid[y][x_word] |= mask;
        } else {
            self.grid[y][x_word] &= !mask;
        }
    }

    fn inc_mod_ten(num: usize) -> usize {
        if num > 8 { 0 } else { num + 1 }
    }
}

impl Default for TwoBitGrid {
    fn default() -> Self {
        TwoBitGrid::new(0, 0)
    }
}

impl IThreeValueGrid for TwoBitGrid {
    fn get_width(&self) -> usize {
        self.x_size
    }

    fn get_height(&self) -> usize {
        self.y_size
    }

    fn is_empty(&self, x: usize, y: usize) -> bool {
        self.retrieve(x, y) == EMPTY
    }

    fn is_blocked(&self, x: usize, y: usize) -> bool {
        self.retrieve(x, y) == BLOCKED
    }

    fn is_weakly_blocked(&self, x: usize, y: usize) -> bool {
        self.retrieve(x, y) == WEAKLY_BLOCKED
    }

    fn in_bounds(&self, x: usize, y: usize) -> bool {
        x < self.x_size && y < self.y_size
    }

    fn reinitialize(&mut self, width: usize, height: usize) {
        let words = if width == 0 { 0 } else { (width + 31) / 32 };
        self.grid = vec![vec![0_u64; words]; height];
        self.x_size = width;
        self.y_size = height;
    }

    fn set_empty(&mut self, x: usize, y: usize) {
        self.set_bits(x, y, false, false);
    }

    fn set_blocked(&mut self, x: usize, y: usize) {
        self.set_bits(x, y, false, true);
    }

    fn set_weakly_blocked(&mut self, x: usize, y: usize) {
        self.set_bits(x, y, true, false);
    }
}

impl std::fmt::Display for TwoBitGrid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut output = String::from(" ");
        let mut count = 0usize;
        for _ in 0..self.x_size {
            output.push_str(&count.to_string());
            count = Self::inc_mod_ten(count);
        }
        output.push('\n');
        count = 0;
        for y in 0..self.y_size {
            output.push_str(&count.to_string());
            count = Self::inc_mod_ten(count);
            for x in 0..self.x_size {
                let item = self.retrieve(x, y);
                if item == EMPTY {
                    output.push('_');
                } else if item == BLOCKED {
                    output.push('X');
                } else {
                    output.push('0');
                }
            }
            output.push('\n');
        }
        if output.ends_with('\n') {
            output.pop();
        }
        write!(f, "{}", output)
    }
}
