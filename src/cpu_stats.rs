use std::process::Command;

pub(crate) fn get_cpu_temperature_c() -> Option<f64> {
    let output = Command::new("/opt/vc/bin/vcgencmd")
        .arg("measure_temp")
        .output()
        .or_else(|_| Command::new("vcgencmd").arg("measure_temp").output());

    let output_str = match output {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
        Ok(output) => {
            log::error!("Command failed with status: {:?}", output.status.code());
            return None;
        }
        Err(err) => {
            log::error!("Failed to execute vcgencmd command: {err}");
            return None;
        }
    };

    let temp_c = output_str
        .strip_prefix("temp=")
        .and_then(|s| s.strip_suffix("'C"))
        .and_then(|temp| temp.parse::<f64>().ok());

    if temp_c.is_none() {
        log::error!("Unexpected output format: {output_str}");
    }

    temp_c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_cpu_temperature_format() {
        // If vcgencmd is available, verify it parses to a realistic temperature
        if let Some(temp) = get_cpu_temperature_c() {
            assert!(
                temp > -20.0 && temp < 120.0,
                "Temperature should be in reasonable range: {temp}°C"
            );
        }
    }
}
