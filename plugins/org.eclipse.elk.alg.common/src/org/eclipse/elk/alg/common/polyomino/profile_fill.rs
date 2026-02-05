use crate::org::eclipse::elk::alg::common::polyomino::structures::PolyominoLike;

pub struct ProfileFill;

impl ProfileFill {
    pub fn fill_polyomino<P: PolyominoLike>(poly: &mut P) {
        let width = poly.get_width();
        let height = poly.get_height();

        let mut north_profile = vec![0_i32; width];
        let mut south_profile = vec![0_i32; width];
        let mut east_profile = vec![0_i32; height];
        let mut west_profile = vec![0_i32; height];

        for xi in 0..width {
            let mut y = 0;
            while y < height && !poly.is_blocked(xi, y) {
                y += 1;
            }
            north_profile[xi] = y as i32;
        }

        for xi in 0..width {
            let mut y = height as i32 - 1;
            while y >= 0 && !poly.is_blocked(xi, y as usize) {
                y -= 1;
            }
            south_profile[xi] = y;
        }

        for yi in 0..height {
            let mut x = 0;
            while x < width && !poly.is_blocked(x, yi) {
                x += 1;
            }
            east_profile[yi] = x as i32;
        }

        for yi in 0..height {
            let mut x = width as i32 - 1;
            while x >= 0 && !poly.is_blocked(x as usize, yi) {
                x -= 1;
            }
            west_profile[yi] = x;
        }

        for xi in 0..width {
            for yi in 0..height {
                if (xi as i32) < west_profile[yi]
                    && (xi as i32) > east_profile[yi]
                    && (yi as i32) < south_profile[xi]
                    && (yi as i32) > north_profile[xi]
                {
                    poly.set_blocked(xi, yi);
                }
            }
        }
    }
}
