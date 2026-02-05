pub mod block;
pub mod block_row;
pub mod block_stack;
pub mod drawing_data;
pub mod drawing_data_descriptor;
pub mod drawing_util;
pub mod rect_row;
pub mod rows_storage;

use std::cell::RefCell;
use std::rc::Rc;

pub use block::Block;
pub use block_row::BlockRow;
pub use block_stack::BlockStack;
pub use drawing_data::DrawingData;
pub use drawing_data_descriptor::DrawingDataDescriptor;
pub use drawing_util::DrawingUtil;
pub use rect_row::RectRow;

pub type BlockRef = Rc<RefCell<Block>>;
pub type BlockStackRef = Rc<RefCell<BlockStack>>;
pub type RectRowRef = Rc<RefCell<RectRow>>;
