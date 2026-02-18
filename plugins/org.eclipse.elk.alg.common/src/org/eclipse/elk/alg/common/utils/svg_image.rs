use std::fs::File;
use std::io::Write;

use org_eclipse_elk_core::org::eclipse::elk::core::math::{ElkRectangle, KVector};

pub struct SVGImage {
    spacing: f64,
    view_box_x: f64,
    view_box_y: f64,
    view_box_w: f64,
    view_box_h: f64,
    view_box_x2: f64,
    view_box_y2: f64,
    enable_update_view_box: bool,
    current: String,
    groups: Vec<(String, String)>,
    pub file_name: Option<String>,
    index: usize,
    pub debug: bool,
    pub max_area: f64,
}

impl SVGImage {
    pub fn new(file: Option<&str>) -> Self {
        let mut image = SVGImage {
            spacing: 20.0,
            view_box_x: f64::INFINITY,
            view_box_y: f64::INFINITY,
            view_box_w: 0.0,
            view_box_h: 0.0,
            view_box_x2: f64::NEG_INFINITY,
            view_box_y2: f64::NEG_INFINITY,
            enable_update_view_box: true,
            current: "main".to_string(),
            groups: Vec::new(),
            file_name: None,
            index: 0,
            debug: false,
            max_area: 8_294_400.0,
        };
        if let Some(file) = file {
            let trimmed = file.trim();
            if !trimmed.is_empty() && !trimmed.starts_with("null") {
                image.file_name = Some(trimmed.to_string());
                image.debug = true;
            }
        }
        image.groups.push((image.current.clone(), String::new()));
        image
    }

    pub fn g(&mut self, key: &str) -> &mut Self {
        if self.debug {
            self.ensure_group(key);
            self.current = key.to_string();
        }
        self
    }

    pub fn add_groups(&mut self, keys: &[&str]) {
        if self.debug {
            for key in keys {
                let idx = self.ensure_group(key);
                self.groups[idx].1.clear();
            }
        }
    }

    pub fn clear_group(&mut self, key: &str) {
        if self.debug {
            if let Some(idx) = self.group_index(key) {
                self.groups[idx].1.clear();
            }
        }
    }

    pub fn remove_group(&mut self, key: &str) {
        if self.debug {
            if key == "main" {
                self.clear_group(key);
            } else if let Some(idx) = self.group_index(key) {
                self.groups.remove(idx);
            }
        }
    }

    pub fn set_view_box(&mut self, x: f64, y: f64, w: f64, h: f64) {
        if self.debug {
            self.view_box_x = x;
            self.view_box_y = y;
            self.view_box_w = w;
            self.view_box_h = h;
            self.enable_update_view_box = false;
        }
    }

    pub fn clear(&mut self) {
        self.debug = self
            .file_name
            .as_ref()
            .map(|name| !name.is_empty())
            .unwrap_or(false);
        if self.debug {
            self.groups.clear();
            self.current = "main".to_string();
            self.groups.push((self.current.clone(), String::new()));
            if self.enable_update_view_box {
                self.view_box_x = f64::INFINITY;
                self.view_box_y = f64::INFINITY;
                self.view_box_w = 0.0;
                self.view_box_h = 0.0;
                self.view_box_x2 = f64::NEG_INFINITY;
                self.view_box_y2 = f64::NEG_INFINITY;
            }
        }
    }

    pub fn add_element_str(&mut self, element: &str) {
        if self.debug {
            let idx = self.ensure_group(&self.current.clone());
            self.groups[idx].1.push_str(element);
            self.groups[idx].1.push('\n');
            self.current = "main".to_string();
        }
    }

    pub fn add_circle(&mut self, x: f64, y: f64) {
        if self.debug {
            self.add_circle_with_attrs(
                x,
                y,
                5.0,
                "stroke=\"black\" stroke-width=\"1\" fill=\"none\"",
            );
        }
    }

    pub fn add_circle_with_attrs(&mut self, x: f64, y: f64, r: f64, attributes: &str) {
        if self.debug {
            self.add_element_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" {} />",
                x, y, r, attributes
            ));
            let p1 = KVector::with_values(x - r, y - r);
            let p2 = KVector::with_values(x + r, y + r);
            self.update_view_box(&[p1, p2]);
        }
    }

    pub fn add_line(&mut self, x1: f64, y1: f64, x2: f64, y2: f64) {
        if self.debug {
            self.add_line_with_attrs(x1, y1, x2, y2, "stroke=\"black\" stroke-width=\"1\"");
        }
    }

    pub fn add_line_with_attrs(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, attributes: &str) {
        if self.debug {
            self.add_element_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" {} />",
                x1, y1, x2, y2, attributes
            ));
            let p1 = KVector::with_values(x1, y1);
            let p2 = KVector::with_values(x2, y2);
            self.update_view_box(&[p1, p2]);
        }
    }

    pub fn add_rect_with_values(&mut self, x: f64, y: f64, w: f64, h: f64, attributes: &str) {
        if self.debug {
            self.add_element_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" {} />",
                x, y, w, h, attributes
            ));
            let p1 = KVector::with_values(x, y);
            let p2 = KVector::with_values(x + w, y + h);
            self.update_view_box(&[p1, p2]);
        }
    }

    pub fn add_rect(&mut self, rect: &ElkRectangle, attributes: &str) {
        if self.debug {
            self.add_rect_with_values(rect.x, rect.y, rect.width, rect.height, attributes);
        }
    }

    pub fn add_poly(&mut self, attributes: &str, points: &[KVector]) {
        if self.debug {
            let mut str_buf = String::from("<polyline points=\"");
            for point in points {
                str_buf.push_str(&format!("{},{} ", point.x, point.y));
            }
            str_buf.push_str(&format!("\" {} />", attributes));
            self.add_element_str(&str_buf);
            self.update_view_box(points);
        }
    }

    pub fn add_text(&mut self, x: f64, y: f64, text: &str, attributes: &str) {
        if self.debug {
            self.add_element_str(&format!(
                "<text x=\"{}\" y=\"{}\" {}>{}</text>",
                x, y, attributes, text
            ));
        }
    }

    fn update_view_box(&mut self, points: &[KVector]) {
        if self.debug && self.enable_update_view_box {
            for point in points {
                if point.x.is_finite() && point.y.is_finite() {
                    self.view_box_x = self.view_box_x.min(point.x);
                    self.view_box_y = self.view_box_y.min(point.y);
                    self.view_box_x2 = self.view_box_x2.max(point.x);
                    self.view_box_y2 = self.view_box_y2.max(point.y);
                }
            }
            self.view_box_w = self.view_box_x2 - self.view_box_x;
            self.view_box_h = self.view_box_y2 - self.view_box_y;
        }
    }

    pub fn save_with_name(&self, file_name: &str) {
        if !self.debug {
            return;
        }
        if let Some(parent) = std::path::Path::new(file_name).parent() {
            if !parent.as_os_str().is_empty() {
                let _ = std::fs::create_dir_all(parent);
            }
        }

        let mut out = match File::create(format!("{}.svg", file_name)) {
            Ok(file) => file,
            Err(_) => return,
        };

        let area = self.view_box_h * self.view_box_w;
        let mut scale = 1.0;
        if area > self.max_area && area.is_finite() {
            scale = (self.max_area / area).sqrt();
        }

        let _ = writeln!(out, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
        let _ = writeln!(
            out,
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100%\" height=\"100%\"  viewBox=\"{} {} {} {}\">",
            self.view_box_x * scale - self.spacing,
            self.view_box_y * scale - self.spacing,
            self.view_box_w * scale + 2.0 * self.spacing,
            self.view_box_h * scale + 2.0 * self.spacing
        );
        for (key, value) in &self.groups {
            let _ = writeln!(
                out,
                "<g transform=\"scale({})\" id=\"{}\">\n{}</g>",
                scale, key, value
            );
        }
        let _ = writeln!(out, "</svg>");
    }

    pub fn save(&self) {
        if let Some(name) = self.file_name.as_ref() {
            self.save_with_name(name);
        }
    }

    pub fn isave_with_name(&mut self, file_name: &str) {
        let name = format!("{}{:03}", file_name, self.index);
        self.save_with_name(&name);
        self.index += 1;
    }

    pub fn isave(&mut self) {
        if let Some(name) = self.file_name.clone() {
            self.isave_with_name(&name);
        }
    }

    fn group_index(&self, key: &str) -> Option<usize> {
        self.groups.iter().position(|(k, _)| k == key)
    }

    fn ensure_group(&mut self, key: &str) -> usize {
        if let Some(idx) = self.group_index(key) {
            idx
        } else {
            self.groups.push((key.to_string(), String::new()));
            self.groups.len() - 1
        }
    }
}

impl Default for SVGImage {
    fn default() -> Self {
        SVGImage::new(None)
    }
}
