import pandas as pd
import numpy as np
import toml

# Read the voltage sheet
df_voltage = pd.read_excel("Sensorboard_SN0.xlsx", sheet_name="Voltage")

# Read the voltage sheet
df_power_forward = pd.read_excel("Sensorboard_SN0.xlsx", sheet_name="RF Forward")

# Read the voltage sheet
df_power_reverse = pd.read_excel("Sensorboard_SN0.xlsx", sheet_name="RF Reverse")

# Define a dictionary with a list
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
        "voltage_main": np.polyfit(df_voltage["Main"], df_voltage["Ground Truth"], 5).tolist()[::-1],
        "voltage_amp": np.polyfit(df_voltage["Amplifier"], df_voltage["Ground Truth"], 5).tolist()[::-1],
        "power_forward": np.polyfit(df_power_forward["Counts (ADC)"], df_power_forward["Ground Truth (W)"], 5).tolist()[::-1],
        "power_reverse": np.polyfit(df_power_reverse["Counts (ADC)"], df_power_reverse["Ground Truth (W)"], 5).tolist()[::-1],
    }
}

# Save to a TOML file
with open("config.toml", "w") as toml_file:
    toml_file.write("# Test comment\n")
    toml.dump(config_dict, toml_file)


print("Data saved to config.toml")
