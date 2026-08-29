// RepeaterPi Sensorboard Modbus Interface (Adafruit QT Py SAMD21)
// Implements the register map defined in README.md
// ("RepeaterPi Sensorboard Modbus Interface Specification").
//
// Requires libraries:
//   - ModbusRTUSlave
//   - TemperatureZero (by Electronic Cats - for SAMD21 die temperature)

#include <ModbusRTUSlave.h>
#include <Wire.h>
#include <TemperatureZero.h>
#include <ArduinoUniqueID.h>

const uint32_t FIXED_BAUD_RATE = 115200UL;
const uint8_t SLAVE_ID = 1;
const unsigned long SAMPLE_INTERVAL_MS = 1000UL;

// Use USB CDC Serial for direct testing via /dev/ttyACM0.
// (Change 'Serial' to 'Serial1' if using an external USB-to-UART adapter on RX/TX pins).
ModbusRTUSlave modbus(Serial);
TemperatureZero TempZero;

bool bmeOk = true;
bool adsOk = true;

// ---------------------------------------------------------------------
// Input registers (FC 04), 0-based relative offsets matching 30001+.
// Fixed 10-register stride per sensor family.
// ---------------------------------------------------------------------
enum {
  IREG_STATUS = 0x0000,              // System Status Word
  IREG_DEVICE_ID = 0x0001,           // 16-bit hash of SAMD21 UniqueID

  IREG_MCU_CORE_TEMP = 0x000A,       // 0.01 degC (SAMD21 Internal Die Temp)
  IREG_MCU_ADC0 = 0x000B,            // 1 mV (QT Py Pin A0)
  IREG_MCU_ADC1 = 0x000C,            // 1 mV (QT Py Pin A1)
  IREG_MCU_ADC2 = 0x000D,            // 1 mV (QT Py Pin A2)
  IREG_MCU_ADC3 = 0x000E,            // 1 mV (QT Py Pin A3)

  IREG_BME_TEMP = 0x0014,            // 0.01 degC
  IREG_BME_HUMIDITY = 0x0015,        // 0.01 %RH
  IREG_BME_PRESSURE_MSW = 0x0016,    // 0.1 Pa, high word
  IREG_BME_PRESSURE_LSW = 0x0017,    // 0.1 Pa, low word

  IREG_ADS_CH0 = 0x001E,             // 0.1 mV
  IREG_ADS_CH1 = 0x001F,
  IREG_ADS_CH2 = 0x0020,
  IREG_ADS_CH3 = 0x0021,

  NUM_INPUT_REGISTERS = 0x0028       // 30001-30040
};

// Status Word bits
enum {
  SYS_READY_BIT = 0,
  BME280_FAULT_BIT = 1,
  ADS1115_FAULT_BIT = 2,
  TEMP_ALERT_BIT = 3,
  ADS_OVERFLOW_BIT = 4,
};

#define TEMP_ALERT_THRESHOLD_C 70.0f
#define SENTINEL_INT16 ((int16_t)0x8000)
#define SENTINEL_UINT16 (0xFFFF)

// Generic stub values used in place of real external sensor reads
#define STUB_TEMP_C 25.0f
#define STUB_HUMIDITY_RH 50.0f
#define STUB_PRESSURE_PA 101325.0f
#define STUB_ADS_MILLIVOLTS 1000.0f

uint16_t inputRegisters[NUM_INPUT_REGISTERS];

unsigned long lastSampleMs = 0;

void writeUint32(uint16_t regs[], uint16_t offset, uint32_t value) {
  regs[offset] = (uint16_t)(value >> 16);
  regs[offset + 1] = (uint16_t)(value & 0xFFFF);
}

void setStatusBit(uint8_t bit, bool set) {
  if (set) {
    inputRegisters[IREG_STATUS] |= (1 << bit);
  } else {
    inputRegisters[IREG_STATUS] &= ~(1 << bit);
  }
}

uint16_t fnv1a16UniqueId() {
  uint32_t hash = 2166136261UL;

  for (size_t i = 0; i < UniqueIDsize; i++) {
    hash ^= UniqueID[i];
    hash *= 16777619UL;
  }

  return (uint16_t)((hash >> 16) ^ (hash & 0xFFFF));
}

void setup() {
  // Begin USB Serial interface at fixed 115200 baud
  Serial.begin(FIXED_BAUD_RATE);

  // Initialize SAMD21 Internal Temperature Sensor
  TempZero.init();

  modbus.configureInputRegisters(inputRegisters, NUM_INPUT_REGISTERS);
  inputRegisters[IREG_DEVICE_ID] = fnv1a16UniqueId();
  
  // Start Modbus RTU at fixed 115200 baud
  modbus.begin(SLAVE_ID, FIXED_BAUD_RATE);

  // Initialize I2C Bus (STEMMA QT Connector)
  Wire.begin();

  setStatusBit(BME280_FAULT_BIT, !bmeOk);
  setStatusBit(ADS1115_FAULT_BIT, !adsOk);
  setStatusBit(SYS_READY_BIT, bmeOk && adsOk);
}

void SampleMCU() {
  // Read SAMD21 internal silicon die temperature
  float coreTempC = TempZero.readInternalTemperature();
  inputRegisters[IREG_MCU_CORE_TEMP] = (int16_t)(coreTempC * 100.0f);
  setStatusBit(TEMP_ALERT_BIT, coreTempC > TEMP_ALERT_THRESHOLD_C);

  // Read QT Py Analog Input Channels (A0, A1, A2, A3)
  const uint8_t adcPins[4] = {A0, A1, A2, A3};
  for (uint8_t i = 0; i < 4; i++) {
    int raw = analogRead(adcPins[i]);
    // Standard 10-bit ADC reference (3300 mV / 1023)
    uint16_t millivolts = (uint16_t)((raw * 3300UL) / 1023UL);
    inputRegisters[IREG_MCU_ADC0 + i] = millivolts;
  }
}

void sampleBme280() {
  if (!bmeOk) {
    inputRegisters[IREG_BME_TEMP] = SENTINEL_INT16;
    inputRegisters[IREG_BME_HUMIDITY] = SENTINEL_UINT16;
    writeUint32(inputRegisters, IREG_BME_PRESSURE_MSW, 0xFFFFFFFF);
    return;
  }

  float tempC = STUB_TEMP_C;
  float humidity = STUB_HUMIDITY_RH;
  float pressurePa = STUB_PRESSURE_PA;

  inputRegisters[IREG_BME_TEMP] = (int16_t)(tempC * 100.0f);
  inputRegisters[IREG_BME_HUMIDITY] = (uint16_t)(humidity * 100.0f);
  writeUint32(inputRegisters, IREG_BME_PRESSURE_MSW, (uint32_t)(pressurePa * 10.0f));
}

void sampleAds1115() {
  if (!adsOk) {
    inputRegisters[IREG_ADS_CH0] = SENTINEL_INT16;
    inputRegisters[IREG_ADS_CH1] = SENTINEL_INT16;
    inputRegisters[IREG_ADS_CH2] = SENTINEL_INT16;
    inputRegisters[IREG_ADS_CH3] = SENTINEL_INT16;
    return;
  }

  for (uint8_t ch = 0; ch < 4; ch++) {
    inputRegisters[IREG_ADS_CH0 + ch] = (int16_t)(STUB_ADS_MILLIVOLTS * 10.0f);
  }

  setStatusBit(ADS_OVERFLOW_BIT, false);
}

void loop() {
  unsigned long now = millis();
  if ((unsigned long)(now - lastSampleMs) >= SAMPLE_INTERVAL_MS) {
    lastSampleMs = now;
    SampleMCU();
    sampleBme280();
    sampleAds1115();
  }

  modbus.poll();
}
