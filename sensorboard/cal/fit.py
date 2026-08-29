import numpy as np
import pandas as pd
import toml
import matplotlib.pyplot as plt
from scipy.interpolate import PchipInterpolator


def fit_pchip_channel(x_data, y_data):
    """Fit PCHIP model on 1D channel data and return dict with x, y, and slope derivative d lists."""
    x_flat = np.ravel(x_data)
    y_flat = np.ravel(y_data)
    sort_idx = np.argsort(x_flat)
    x_sorted = x_flat[sort_idx]
    y_sorted = y_flat[sort_idx]

    pchip = PchipInterpolator(x_sorted, y_sorted, extrapolate=True)
    d_slopes = pchip.derivative(1)(x_sorted)

    return {
        "x": [round(float(val), 6) for val in x_sorted],
        "y": [round(float(val), 8) for val in y_sorted],
        "d": [round(float(val), 10) for val in d_slopes],
    }


def plot_channel(ax, x_data, y_data, title, xlabel, ylabel, color="#1f77b4"):
    """Plot measured knot points and fitted PCHIP interpolation curve on specified subplot axis."""
    x_flat = np.ravel(x_data)
    y_flat = np.ravel(y_data)
    sort_idx = np.argsort(x_flat)
    x_sorted = x_flat[sort_idx]
    y_sorted = y_flat[sort_idx]

    pchip = PchipInterpolator(x_sorted, y_sorted, extrapolate=True)
    x_grid = np.linspace(min(x_sorted), max(x_sorted), 500)
    y_grid = pchip(x_grid)

    ax.plot(x_grid, y_grid, color=color, linewidth=2.0, label="PCHIP Fit")
    ax.scatter(x_sorted, y_sorted, color="black", s=20, label="Knot Points", zorder=5)

    ax.set_title(title, fontsize=11, fontweight="bold")
    ax.set_xlabel(xlabel, fontsize=9)
    ax.set_ylabel(ylabel, fontsize=9)
    ax.grid(True, linestyle="--", alpha=0.6)
    ax.legend(loc="lower right", fontsize=8)


# Load Excel sheets
excel_file = "Sensorboard_SN0.xlsx"
df_voltage = pd.read_excel(excel_file, sheet_name="Voltage")
df_power_forward = pd.read_excel(excel_file, sheet_name="RF Forward")
df_power_reverse = pd.read_excel(excel_file, sheet_name="RF Reverse")

# 5V / USB Voltage Divider circuit parameters (10k / 2.2k divider on 10-bit ADC)
R1 = 10000.0      # 10k resistor (ohms)
R2 = 2200.0       # 2.2k resistor (ohms)
V_ref = 3.3       # 3.3V ADC reference voltage
ADC_max = 1023.0  # 10-bit ADC resolution max count

# Exact physical slope (V / ADC count) = (V_ref / ADC_max) * ((R1 + R2) / R2)
usb_slope = (V_ref / ADC_max) * ((R1 + R2) / R2)
usb_max_counts = 5.0 / usb_slope  # ADC counts at 5.0V

# Construct configuration dictionary containing exclusively PCHIP calibration parameters
config_dict = {
    "influxdb": {
        "endpoint": "https://influxdb.kg5key.com:443",
        "database_name": "repeaterpi",
        "token": "REPLACEME",
        "site_name": "kg5key",
    },
    "serial": {
        "port": "/dev/ttyACM0",
        "baud": 9600,
    },
    "calibration": {
        "voltage_clamp": 1.0,
        "power_forward_clamp": 0.1,
        "power_reverse_clamp": 0.1,
        "voltage_main": fit_pchip_channel(df_voltage["Main"], df_voltage["Ground Truth"]),
        "voltage_amp": fit_pchip_channel(df_voltage["Amplifier"], df_voltage["Ground Truth"]),
        "voltage_usb": fit_pchip_channel([0.0, usb_max_counts], [0.0, 5.0]),
        "power_forward": fit_pchip_channel(df_power_forward["Raw (Counts)"], df_power_forward["Ground Truth (W)"]),
        "power_reverse": fit_pchip_channel(df_power_reverse["Raw (Counts)"], df_power_reverse["Ground Truth (W)"]),
    },
}


# Save to TOML file
output_file = "config.toml"
with open(output_file, "w") as f:
    f.write("# Sensorboard Calibration Configuration (PCHIP Parameters)\n")
    toml.dump(config_dict, f)

print(f"PCHIP parameters successfully computed and saved to {output_file}")


# Generate 2x2 Calibration Plots (Primary, Amp, Forward, Reverse)
fig, axes = plt.subplots(2, 2, figsize=(12, 9))

# 1. Primary Voltage (Main Channel)
plot_channel(
    axes[0, 0],
    df_voltage["Main"],
    df_voltage["Ground Truth"],
    title="Primary Voltage Calibration",
    xlabel="Raw Main ADC Counts",
    ylabel="Ground Truth Voltage (V)",
    color="#1f77b4",
)

# 2. Amp Voltage (Amplifier Channel)
plot_channel(
    axes[0, 1],
    df_voltage["Amplifier"],
    df_voltage["Ground Truth"],
    title="Amp Voltage Calibration",
    xlabel="Raw Amp ADC Counts",
    ylabel="Ground Truth Voltage (V)",
    color="#2ca02c",
)

# 3. Forward Power (RF Forward Channel)
plot_channel(
    axes[1, 0],
    df_power_forward["Raw (Counts)"],
    df_power_forward["Ground Truth (W)"],
    title="Forward Power Calibration",
    xlabel="Raw Forward ADC Counts",
    ylabel="Ground Truth Power (W)",
    color="#ff7f0e",
)

# 4. Reverse Power (RF Reverse Channel)
plot_channel(
    axes[1, 1],
    df_power_reverse["Raw (Counts)"],
    df_power_reverse["Ground Truth (W)"],
    title="Reverse Power Calibration",
    xlabel="Raw Reverse ADC Counts",
    ylabel="Ground Truth Power (W)",
    color="#9467bd",
)

plt.tight_layout()
plot_filename = "calibration_plots.png"
plt.savefig(plot_filename, dpi=300)
print(f"2x2 calibration plot saved to {plot_filename}")
