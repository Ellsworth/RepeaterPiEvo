# RepeaterPi Sensorboard Modbus Interface Specification

**Document Version:** 1.0.0

**Target Hardware:** RP2350 / SAMD21 Microcontroller

**Sensors:** Bosch BME280, TI ADS1115, MCU Internal Temperature & ADC

**Protocol:** Modbus RTU (RS-485 / UART)

---

### 1. Executive Overview

This specification defines the Modbus RTU communication interface for the **RepeaterPi Sensorboard**. The board collects environmental metrics (temperature, humidity, pressure) and analog voltage telemetry (DC bus voltages, battery monitoring, auxiliary inputs) to expose them over Modbus RTU for remote monitoring by the RepeaterPi host system.

To ensure future software compatibility and enable modular hardware upgrades without breaking client software, the Input Register map utilizes a **Fixed 10-Register Stride Architecture**. Each sensor family is assigned a static base offset aligned to a 10-register boundary (`+10` stride).

---

### 2. Communications Layer

| Parameter | Configuration / Value |
| --- | --- |
| **Interface** | RS-485 (Half-Duplex) / Direct UART |
| **Protocol** | Modbus RTU |
| **Default Slave Address** | `1` (Configurable via Holding Register `40001`) |
| **Baud Rate** | `115200 bps` (Default) |
| **Data Format** | 8 Data Bits, No Parity, 1 Stop Bit (`8N1`) |
| **Supported Function Codes** | `0x04` (Read Input Registers), `0x03` (Read Holding Registers), `0x06` (Write Single Register), `0x10` (Write Multiple Registers) |

---

### 3. Data Types & Representation

* **`UINT16`:** 16-bit unsigned integer ($0\text{ to }65535$).
* **`INT16`:** 16-bit signed integer in 2's complement ($-32768\text{ to }+32767$).
* **`UINT32`:** 32-bit unsigned integer occupying **two contiguous 16-bit registers** in Big-Endian Word Order (**MSW First, LSW Second**).
* **Sentinel Values:** An invalid or uninitialized sensor reading returns `0x8000` ($-32768$) for signed registers or `0xFFFF` for unsigned registers.
* **Gap Behavior:** Reading reserved registers within the stride space returns `0x0000`.

---

### 4. Input Register Map (`Function Code 04`)

Base Address Offset: `30001` (1-Based) / `0x0000` (0-Based Relative Address).

```
   Address Space    | Stride Block
 -------------------+-----------------------------
  30001 - 30010     | System Status & Diagnostics
  30011 - 30020     | MCU Local Telemetry
  30021 - 30030     | BME280 Environmental Data
  30031 - 30040     | ADS1115 ADC Channels
```

#### Complete Register Directory

| Register Address | Relative Offset | Parameter | Type | Units / Scale | Description |
| --- | --- | --- | --- | --- | --- |
| **`30001`** | `0x0000` | System Status Word | `UINT16` | Bitfield | Sensor health and system flags (See Sec. 4.1) |
| **`30002`** | `0x0001` | Device ID | `UINT16` | Hash | 16-bit FNV-1a hash of MCU UniqueID |
| `30003`..`30010` | `0x0002`..`0x0009` | *Reserved* | — | — | Reserved for system diagnostics expansion |
|  |  |  |  |  |  |
| **`30011`** | `0x000A` | MCU Core Temp | `INT16` | $0.01\,^\circ\text{C}$ | Die temperature ($2540 = 25.40\,^\circ\text{C}$) |
| **`30012`** | `0x000B` | MCU ADC Channel 0 | `UINT16` | $1\text{ mV}$ | Voltage on ADC0 ($0\text{ to }3300\text{ mV}$) |
| **`30013`** | `0x000C` | MCU ADC Channel 1 | `UINT16` | $1\text{ mV}$ | Voltage on ADC1 |
| **`30014`** | `0x000D` | MCU ADC Channel 2 | `UINT16` | $1\text{ mV}$ | Voltage on ADC2 |
| **`30015`** | `0x000E` | MCU ADC Channel 3 | `UINT16` | $1\text{ mV}$ | Voltage on ADC3 |
| `30016`..`30020` | `0x000F`..`0x0013` | *Reserved* | — | — | Reserved for extra ADC pins |
|  |  |  |  |  |  |
| **`30021`** | `0x0014` | BME280 Temperature | `INT16` | $0.01\,^\circ\text{C}$ | Ambient temperature ($-4000\text{ to }+8500$) |
| **`30022`** | `0x0015` | BME280 Relative Humidity | `UINT16` | $0.01\%\text{ RH}$ | Relative humidity ($4550 = 45.50\%$) |
| **`30023`** | `0x0016` | BME280 Pressure (MSW) | `UINT32` | $0.1\text{ Pa}$ | Barometric pressure high word |
| **`30024`** | `0x0017` | BME280 Pressure (LSW) | `UINT32` | $0.1\text{ Pa}$ | Barometric pressure low word ($1013250 = 1013.25\text{ hPa}$) |
| `30025`..`30030` | `0x0018`..`0x001D` | *Reserved* | — | — | Reserved for calculated dew point / air metrics |
|  |  |  |  |  |  |
| **`30031`** | `0x001E` | ADS1115 Channel 0 | `INT16` | $0.1\text{ mV}$ | Scaled voltage on AIN0 ($12000 = 1200.0\text{ mV}$) |
| **`30032`** | `0x001F` | ADS1115 Channel 1 | `INT16` | $0.1\text{ mV}$ | Scaled voltage on AIN1 |
| **`30033`** | `0x0020` | ADS1115 Channel 2 | `INT16` | $0.1\text{ mV}$ | Scaled voltage on AIN2 |
| **`30034`** | `0x0021` | ADS1115 Channel 3 | `INT16` | $0.1\text{ mV}$ | Scaled voltage on AIN3 |
| `30035`..`30040` | `0x0022`..`0x0027` | *Reserved* | — | — | Reserved for differential/raw ADS1115 modes |

---

#### 4.1 System Status Word (`30001`) Bitfield Definitions

| Bit | Name | Description when Bit = 1 |
| --- | --- | --- |
| **0** | `SYS_READY` | System booted and sensors initialized successfully |
| **1** | `BME280_FAULT` | I2C communication error or missing BME280 |
| **2** | `ADS1115_FAULT` | I2C communication error or missing ADS1115 |
| **3** | `TEMP_ALERT` | MCU die temperature exceeds warning threshold ($> 70\,^\circ\text{C}$) |
| **4** | `ADS_OVERFLOW` | ADS1115 input voltage exceeded selected PGA range |
| **5–15** | `RESERVED` | Always set to `0` |

---

### 5. Holding Register Map (`Function Codes 03, 06, 16`)

Base Address Offset: `40001` (1-Based) / `0x0000` (0-Based Relative Address).

Holding registers store operating settings in MCU flash memory.

| Register | Relative Offset | Parameter | Default | Range / Decoding |
| --- | --- | --- | --- | --- |
| **`40001`** | `0x0000` | Modbus Slave Address | `1` | $1\text{ to }247$ |
| **`40002`** | `0x0001` | Baud Rate Config | `3` | `0`=9600, `1`=19200, `2`=57600, `3`=115200 |
| **`40003`** | `0x0002` | ADS1115 PGA Gain | `1` | `0`=$\pm 6.144\text{V}$, `1`=$\pm 4.096\text{V}$, `2`=$\pm 2.048\text{V}$ |
| **`40004`** | `0x0003` | Sample Rate | `1000` | Sensor polling interval in milliseconds ($100\text{ to }60000\text{ ms}$) |
| **`40005`** | `0x0004` | Save Configuration | `0` | Write `0x55AA` to trigger flash write of settings |

---

### 6. Polling Example (Python / `pymodbus`)

Because all 4 sensor families reside in a contiguous block of 40 registers (`30001` to `30040`), the host software can retrieve all telemetry in a single Modbus read request:

```python
from pymodbus.client import ModbusSerialClient

client = ModbusSerialClient(port='/dev/ttyACM0', baudrate=115200, timeout=1)
client.connect()

# Read 40 input registers starting at address 0x0000 (30001)
rr = client.read_input_registers(address=0x0000, count=40, slave=1)

if not rr.isError():
    status       = rr.registers[0]
    core_temp    = rr.registers[10] / 100.0  # Reg 30011
    adc0         = rr.registers[11]          # Reg 30012
    bme_temp     = rr.registers[20] / 100.0  # Reg 30021
    bme_hum      = rr.registers[21] / 100.0  # Reg 30022
    bme_press    = ((rr.registers[22] << 16) | rr.registers[23]) / 10.0  # Reg 30023-30024
    ads_ch0      = rr.registers[30] / 10.0   # Reg 30031

    print(f"Status: {hex(status)} | MCU Temp: {core_temp}°C | BME280: {bme_temp}°C, {bme_hum}%, {bme_press}Pa")
```
