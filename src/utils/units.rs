// Преобразования единиц измерения: импульсы ↔ мм, градусы

/// Преобразует импульсы в миллиметры
pub fn pulses_to_mm(pulses: f64, pulses_per_mm: f64) -> f64 {
    pulses / pulses_per_mm
}

/// Преобразует миллиметры в импульсы
pub fn mm_to_pulses(mm: f64, pulses_per_mm: f64) -> f64 {
    mm * pulses_per_mm
}

/// Преобразует импульсы энкодера в градусы
pub fn pulses_to_degrees(pulses: f64, pulses_per_degree: f64) -> f64 {
    pulses / pulses_per_degree
}

/// Преобразует градусы в импульсы энкодера
pub fn degrees_to_pulses(degrees: f64, pulses_per_degree: f64) -> f64 {
    degrees * pulses_per_degree
}

/// Рассчитывает pulses_per_mm на основе параметров шагового двигателя
pub fn calculate_pulses_per_mm(
    steps_per_rev: u32,
    lead_screw_pitch_mm: f64,
    microsteps: u32,
) -> f64 {
    (steps_per_rev * microsteps) as f64 / lead_screw_pitch_mm
}

/// Рассчитывает pulses_per_degree на основе параметров энкодера
pub fn calculate_pulses_per_degree(encoder_cpr: u32) -> f64 {
    encoder_cpr as f64 / 360.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pulses_to_mm() {
        let pulses_per_mm = 3200.0;
        let pulses = 3200.0;
        let mm = pulses_to_mm(pulses, pulses_per_mm);
        assert!((mm - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_mm_to_pulses() {
        let pulses_per_mm = 3200.0;
        let mm = 1.0;
        let pulses = mm_to_pulses(mm, pulses_per_mm);
        assert!((pulses - 3200.0).abs() < 0.001);
    }

    #[test]
    fn test_calculate_pulses_per_mm() {
        let steps_per_rev = 200;
        let lead_screw_pitch = 8.0;
        let microsteps = 16;
        let result = calculate_pulses_per_mm(steps_per_rev, lead_screw_pitch, microsteps);
        assert!((result - 400.0).abs() < 0.001);
    }
}
