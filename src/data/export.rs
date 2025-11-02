// Экспорт данных сканирования в markdown и CSV форматы

use crate::processing::TemperatureMeasurement;
use std::fs::OpenOptions;
use std::io::{Write, Result as IoResult};
use std::path::Path;

/// Экспортер данных сканирования
pub struct DataExporter {
    output_dir: std::path::PathBuf,
}

impl DataExporter {
    /// Создает новый экспортер данных
    pub fn new(output_dir: impl AsRef<Path>) -> Self {
        let output_dir = output_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&output_dir).ok();
        
        Self { output_dir }
    }

    /// Экспортирует данные в markdown формат (temperatur_reader.md)
    pub fn export_markdown(&self, measurements: &[TemperatureMeasurement]) -> IoResult<()> {
        let file_path = self.output_dir.join("temperatur_reader.md");
        
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&file_path)?;

        writeln!(file, "# Temperature Scan Results\n")?;
        writeln!(file, "Total measurements: {}\n", measurements.len())?;
        writeln!(file, "| X (mm) | Y (mm) | Angle (d:m:s) | T_min (°C) | T_max (°C) |")?;
        writeln!(file, "|--------|--------|---------------|------------|------------|")?;

        for meas in measurements {
            writeln!(
                file,
                "| {:.2} | {:.2} | {} | {:.2} | {:.2} |",
                meas.x_mm,
                meas.y_mm,
                meas.angle_dms,
                meas.temp_min,
                meas.temp_max
            )?;
        }

        Ok(())
    }

    /// Экспортирует данные в CSV формат (temperatur_reader.txt)
    pub fn export_csv(&self, measurements: &[TemperatureMeasurement]) -> IoResult<()> {
        let file_path = self.output_dir.join("temperatur_reader.txt");
        
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&file_path)?;

        // Заголовок CSV
        writeln!(file, "X_mm,Y_mm,Angle_d:m:s,T_max,T_min")?;

        // Данные
        for meas in measurements {
            writeln!(
                file,
                "{:.2},{:.2},{},{:.2},{:.2}",
                meas.x_mm,
                meas.y_mm,
                meas.angle_dms,
                meas.temp_max,
                meas.temp_min
            )?;
        }

        Ok(())
    }

    /// Экспортирует данные в оба формата одновременно
    pub fn export_all(&self, measurements: &[TemperatureMeasurement]) -> IoResult<()> {
        self.export_markdown(measurements)?;
        self.export_csv(measurements)?;
        Ok(())
    }

    /// Добавляет измерение к существующему файлу (append mode)
    pub fn append_measurement(&self, measurement: &TemperatureMeasurement) -> IoResult<()> {
        let file_path = self.output_dir.join("temperatur_reader.txt");
        
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)?;

        writeln!(
            file,
            "{:.2},{:.2},{},{:.2},{:.2}",
            measurement.x_mm,
            measurement.y_mm,
            measurement.angle_dms,
            measurement.temp_max,
            measurement.temp_min
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    #[test]
    fn test_export_markdown() {
        let temp_dir = std::env::temp_dir().join("test_export");
        let exporter = DataExporter::new(&temp_dir);
        
        let measurements = vec![
            TemperatureMeasurement {
                x_mm: 10.0,
                y_mm: 20.0,
                angle_dms: "0:1:0".to_string(),
                temp_min: 25.0,
                temp_max: 30.0,
                timestamp: SystemTime::now(),
                pixel_x: 100,
                pixel_y: 200,
            },
        ];
        
        assert!(exporter.export_markdown(&measurements).is_ok());
    }

    #[test]
    fn test_export_csv() {
        let temp_dir = std::env::temp_dir().join("test_export_csv");
        let exporter = DataExporter::new(&temp_dir);
        
        let measurements = vec![
            TemperatureMeasurement {
                x_mm: 10.0,
                y_mm: 20.0,
                angle_dms: "0:1:0".to_string(),
                temp_min: 25.0,
                temp_max: 30.0,
                timestamp: SystemTime::now(),
                pixel_x: 100,
                pixel_y: 200,
            },
        ];
        
        assert!(exporter.export_csv(&measurements).is_ok());
    }
}

