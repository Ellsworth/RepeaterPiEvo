use std::fs;

/// Reads the CPU/SoC temperature in degrees Celsius from the Linux thermal sysfs interface.
pub(crate) fn get_cpu_temperature_c() -> Option<f64> {
    match fs::read_to_string("/sys/class/thermal/thermal_zone0/temp") {
        Ok(content) => match content.trim().parse::<f64>() {
            Ok(milli_c) => Some(milli_c / 1000.0),
            Err(err) => {
                log::error!("Failed to parse CPU temperature from sysfs: {err}");
                None
            }
        },
        Err(err) => {
            log::error!("Failed to read CPU temperature from sysfs: {err}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_cpu_temperature_format() {
        // If running on a Linux system with thermal_zone0, verify it parses to a realistic temperature
        if let Some(temp) = get_cpu_temperature_c() {
            assert!(
                temp > -20.0 && temp < 120.0,
                "Temperature should be in reasonable range: {temp}°C"
            );
        }
    }
}
