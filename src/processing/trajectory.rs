// Генератор траекторий сканирования: концентрические окружности

use crate::utils::AngleDMS;
use std::f64::consts::PI;

/// Точка траектории сканирования
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScanPoint {
    pub x_mm: f64,
    pub y_mm: f64,
    pub radius_mm: f64,
    pub angle_deg: f64,
    pub angle_dms: AngleDMS,
}

/// Генератор траекторий сканирования
pub struct TrajectoryGenerator {
    center_x: f64,
    center_y: f64,
    max_radius: f64,
    pitch_mm: f64,
    angle_dms: AngleDMS,
}

impl TrajectoryGenerator {
    /// Создает новый генератор траекторий
    pub fn new(
        center_x: f64,
        center_y: f64,
        max_radius: f64,
        pitch_mm: f64,
        angle: AngleDMS,
    ) -> Self {
        Self {
            center_x,
            center_y,
            max_radius,
            pitch_mm,
            angle_dms: angle,
        }
    }

    /// Генерирует концентрические окружности от центра
    /// Возвращает точки траектории в порядке от центра наружу
    pub fn generate_concentric_circles(&self) -> Vec<ScanPoint> {
        let mut points = Vec::new();
        let num_rings = (self.max_radius / self.pitch_mm).ceil() as usize;

        for ring in 1..=num_rings {
            let radius = ring as f64 * self.pitch_mm;
            if radius > self.max_radius {
                break;
            }

            // Рассчитываем количество точек на окружности
            // Минимум 6 точек, максимум определяется шагом
            let circumference = 2.0 * PI * radius;
            let num_points = (circumference / self.pitch_mm).ceil() as usize;
            let num_points = num_points.max(6); // Минимум 6 точек на окружности

            // Генерируем точки на окружности
            for i in 0..num_points {
                let angle_rad = 2.0 * PI * i as f64 / num_points as f64;
                let x = self.center_x + radius * angle_rad.cos();
                let y = self.center_y + radius * angle_rad.sin();

                // Применяем поворот на заданный угол
                let (x_rotated, y_rotated) =
                    self.rotate_point(x, y, self.angle_dms.to_degrees().to_radians());

                points.push(ScanPoint {
                    x_mm: x_rotated,
                    y_mm: y_rotated,
                    radius_mm: radius,
                    angle_deg: angle_rad.to_degrees(),
                    angle_dms: AngleDMS::from_degrees(angle_rad.to_degrees()),
                });
            }
        }

        points
    }

    /// Поворачивает точку вокруг центра на заданный угол
    fn rotate_point(&self, x: f64, y: f64, angle_rad: f64) -> (f64, f64) {
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();

        let dx = x - self.center_x;
        let dy = y - self.center_y;

        let x_rotated = self.center_x + dx * cos_a - dy * sin_a;
        let y_rotated = self.center_y + dx * sin_a + dy * cos_a;

        (x_rotated, y_rotated)
    }

    /// Генерирует спиральную траекторию
    pub fn generate_spiral(&self) -> Vec<ScanPoint> {
        let mut points = Vec::new();
        let mut radius = self.pitch_mm;
        let mut angle: f64 = 0.0;

        while radius <= self.max_radius {
            let x = self.center_x + radius * angle.cos();
            let y = self.center_y + radius * angle.sin();

            // Применяем поворот
            let (x_rotated, y_rotated) =
                self.rotate_point(x, y, self.angle_dms.to_degrees().to_radians());

            points.push(ScanPoint {
                x_mm: x_rotated,
                y_mm: y_rotated,
                radius_mm: radius,
                angle_deg: angle.to_degrees(),
                angle_dms: AngleDMS::from_degrees(angle.to_degrees()),
            });

            // Увеличиваем угол и радиус для спирали
            angle += self.pitch_mm / radius;
            radius += self.pitch_mm * (2.0 * PI / (self.max_radius / self.pitch_mm));
        }

        points
    }

    /// Генерирует сетку траектории
    pub fn generate_grid(&self, step_x: f64, step_y: f64) -> Vec<ScanPoint> {
        let mut points = Vec::new();
        let num_x = ((2.0 * self.max_radius) / step_x).ceil() as usize;
        let num_y = ((2.0 * self.max_radius) / step_y).ceil() as usize;

        for i in 0..num_x {
            let x = self.center_x - self.max_radius + i as f64 * step_x;
            for j in 0..num_y {
                let y = self.center_y - self.max_radius + j as f64 * step_y;

                let dx = x - self.center_x;
                let dy = y - self.center_y;
                let dist = (dx * dx + dy * dy).sqrt();

                if dist <= self.max_radius {
                    points.push(ScanPoint {
                        x_mm: x,
                        y_mm: y,
                        radius_mm: dist,
                        angle_deg: dy.atan2(dx).to_degrees(),
                        angle_dms: AngleDMS::from_degrees(dy.atan2(dx).to_degrees()),
                    });
                }
            }
        }

        points
    }

    /// Получает общее количество точек для концентрических окружностей
    pub fn estimate_point_count(&self) -> usize {
        let num_rings = (self.max_radius / self.pitch_mm).ceil() as usize;
        let mut total = 0;

        for ring in 1..=num_rings {
            let radius = ring as f64 * self.pitch_mm;
            if radius > self.max_radius {
                break;
            }
            let circumference = 2.0 * PI * radius;
            let num_points = (circumference / self.pitch_mm).ceil() as usize;
            total += num_points.max(6);
        }

        total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_concentric_circles() {
        let generator =
            TrajectoryGenerator::new(0.0, 0.0, 10.0, 2.0, AngleDMS::from_str("0:0:0").unwrap());
        let points = generator.generate_concentric_circles();
        assert!(!points.is_empty());
        assert!(points
            .iter()
            .all(|p| (p.x_mm * p.x_mm + p.y_mm * p.y_mm).sqrt() <= 10.0 + 0.1));
    }

    #[test]
    fn test_estimate_point_count() {
        let generator =
            TrajectoryGenerator::new(0.0, 0.0, 10.0, 2.0, AngleDMS::from_str("0:0:0").unwrap());
        let estimated = generator.estimate_point_count();
        let actual = generator.generate_concentric_circles().len();
        assert_eq!(estimated, actual);
    }
}
