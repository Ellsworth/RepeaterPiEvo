# /// script
# requires-python = ">=3.8"
# dependencies = [
#     "pymodbus>=3.0.0",
#     "pyserial>=3.5",
# ]
# ///

import time
from pymodbus.client import ModbusSerialClient
from pymodbus.exceptions import ModbusException

# ---------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------
PORT = "/dev/ttyACM0"  # Adjust for your OS / serial adapter (e.g., COM3 on Windows)
BAUDRATE = 115200
SLAVE_ID = 1

# ---------------------------------------------------------------------
# Helper Functions
# ---------------------------------------------------------------------
def parse_status_word(status):
    """Decode bit flags in Register 30001 (Offset 0)"""
    sys_ready    = bool(status & (1 << 0))
    bme_fault    = bool(status & (1 << 1))
    ads_fault    = bool(status & (1 << 2))
    temp_alert   = bool(status & (1 << 3))
    ads_overflow = bool(status & (1 << 4))
    
    return (
        f"READY: {sys_ready} | "
        f"BME Fault: {bme_fault} | "
        f"ADS Fault: {ads_fault} | "
        f"Over-Temp: {temp_alert} | "
        f"ADS Overflow: {ads_overflow}"
    )

def read_uint32(registers, offset):
    """Combine two 16-bit registers into a 32-bit uint (MSW first)"""
    return (registers[offset] << 16) | registers[offset + 1]


# ---------------------------------------------------------------------
# Main Execution Loop
# ---------------------------------------------------------------------
def main():
    client = ModbusSerialClient(
        port=PORT,
        baudrate=BAUDRATE,
        parity='N',
        stopbits=1,
        bytesize=8,
        timeout=1.0
    )

    if not client.connect():
        print(f"[-] Failed to connect to serial port {PORT}")
        return

    print(f"[+] Connected to {PORT} at {BAUDRATE} baud.\n")

    try:
        while True:
            # ---------------------------------------------------------
            # Read Input Registers (FC 04, 40 registers total, offset 0)
            # ---------------------------------------------------------
            result = client.read_input_registers(address=0, count=40, device_id=SLAVE_ID)
            
            if result.isError():
                print(f"[-] Modbus Error: {result}")
            else:
                regs = result.registers

                # 1. Status Word (0x0000)
                status_raw = regs[0]
                device_id = regs[1]
                status_str = parse_status_word(status_raw)

                # 2. MCU Core & ADC Channels (0x000A - 0x000E)
                # Raw int16 to Signed float conversion for temperature
                core_temp_raw = regs[10] if regs[10] < 0x8000 else regs[10] - 0x10000
                core_temp = core_temp_raw / 100.0
                
                adc0 = regs[11]
                adc1 = regs[12]
                adc2 = regs[13]
                adc3 = regs[14]

                # 3. BME280 Registers (0x0014 - 0x0017)
                bme_temp_raw = regs[20] if regs[20] < 0x8000 else regs[20] - 0x10000
                bme_temp = bme_temp_raw / 100.0 if regs[20] != 0x8000 else None
                bme_hum  = (regs[21] / 100.0) if regs[21] != 0xFFFF else None
                
                press_raw = read_uint32(regs, 22)
                bme_press = (press_raw / 10.0) if press_raw != 0xFFFFFFFF else None

                # 4. ADS1115 Channels (0x001E - 0x0021)
                ads_ch0 = regs[30] / 10.0
                ads_ch1 = regs[31] / 10.0
                ads_ch2 = regs[32] / 10.0
                ads_ch3 = regs[33] / 10.0

                # Clear terminal and print clean output
                print("\033[H\033[J", end="")  # ANSI clear screen
                print("==================================================")
                print(" REPEATERPI SENSORBOARD MODBUS MONITOR (SAMD21)   ")
                print("==================================================")
                print(f"Status Word (0x00) : 0x{status_raw:04X}")
                print(f"Device ID  (0x01) : 0x{device_id:04X}")
                print(f"  └─ Flags         : {status_str}\n")
                
                print(f"SAMD21 Core Temp   : {core_temp:.2f} °C")
                print(f"SAMD21 ADCs (mV)   : A0={adc0} | A1={adc1} | A2={adc2} | A3={adc3}\n")

                if bme_temp is not None:
                    print(f"BME280             : {bme_temp:.2f} °C | {bme_hum:.1f} %RH | {bme_press:.1f} Pa")
                else:
                    print("BME280             : [FAULT / NO SENSOR]")

                print(f"ADS1115 (mV)       : CH0={ads_ch0:.1f} | CH1={ads_ch1:.1f} | CH2={ads_ch2:.1f} | CH3={ads_ch3:.1f}")
                print("==================================================")

            time.sleep(1.0)

    except KeyboardInterrupt:
        print("\n[+] Exiting test script.")
    finally:
        client.close()

if __name__ == "__main__":
    main()
