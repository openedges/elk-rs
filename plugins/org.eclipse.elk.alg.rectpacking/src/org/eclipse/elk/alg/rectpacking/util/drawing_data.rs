use super::drawing_data_descriptor::DrawingDataDescriptor;
use super::drawing_util::DrawingUtil;

#[derive(Clone, Debug)]
pub struct DrawingData {
    scale_measure: f64,
    drawing_width: f64,
    drawing_height: f64,
    area: f64,
    aspect_ratio: f64,
    desired_aspect_ratio: f64,
    placement_option: DrawingDataDescriptor,
    next_x_coordinate: f64,
    next_y_coordinate: f64,
}

impl DrawingData {
    pub fn new(
        desired_aspect_ratio: f64,
        drawing_width: f64,
        drawing_height: f64,
        placement_option: DrawingDataDescriptor,
    ) -> Self {
        Self::with_coordinates(
            desired_aspect_ratio,
            drawing_width,
            drawing_height,
            placement_option,
            0.0,
            0.0,
        )
    }

    pub fn with_coordinates(
        desired_aspect_ratio: f64,
        drawing_width: f64,
        drawing_height: f64,
        placement_option: DrawingDataDescriptor,
        next_x_coordinate: f64,
        next_y_coordinate: f64,
    ) -> Self {
        let mut data = DrawingData {
            scale_measure: 0.0,
            drawing_width,
            drawing_height,
            area: 0.0,
            aspect_ratio: 0.0,
            desired_aspect_ratio,
            placement_option,
            next_x_coordinate,
            next_y_coordinate,
        };
        data.calc_area_aspect_ratio_scale_measure();
        data
    }

    fn calc_area_aspect_ratio_scale_measure(&mut self) {
        if self.drawing_width > 0.0 && self.drawing_height > 0.0 {
            self.area = self.drawing_width * self.drawing_height;
            self.aspect_ratio = self.drawing_width / self.drawing_height;
            self.scale_measure = DrawingUtil::compute_scale_measure(
                self.drawing_width,
                self.drawing_height,
                self.desired_aspect_ratio,
            );
        }
    }

    pub fn drawing_width(&self) -> f64 {
        self.drawing_width
    }

    pub fn set_drawing_width(&mut self, drawing_width: f64) {
        self.drawing_width = drawing_width;
        self.calc_area_aspect_ratio_scale_measure();
    }

    pub fn drawing_height(&self) -> f64 {
        self.drawing_height
    }

    pub fn set_drawing_height(&mut self, drawing_height: f64) {
        self.drawing_height = drawing_height;
        self.calc_area_aspect_ratio_scale_measure();
    }

    pub fn scale_measure(&self) -> f64 {
        self.scale_measure
    }

    pub fn area(&self) -> f64 {
        self.area
    }

    pub fn aspect_ratio(&self) -> f64 {
        self.aspect_ratio
    }

    pub fn placement_option(&self) -> DrawingDataDescriptor {
        self.placement_option
    }

    pub fn set_placement_option(&mut self, placement_option: DrawingDataDescriptor) {
        self.placement_option = placement_option;
    }

    pub fn next_x_coordinate(&self) -> f64 {
        self.next_x_coordinate
    }

    pub fn set_next_x_coordinate(&mut self, value: f64) {
        self.next_x_coordinate = value;
    }

    pub fn next_y_coordinate(&self) -> f64 {
        self.next_y_coordinate
    }

    pub fn set_next_y_coordinate(&mut self, value: f64) {
        self.next_y_coordinate = value;
    }

    pub fn desired_aspect_ratio(&self) -> f64 {
        self.desired_aspect_ratio
    }
}
