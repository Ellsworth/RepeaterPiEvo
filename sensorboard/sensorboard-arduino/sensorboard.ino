#include <Wire.h>
#include <Adafruit_BMP280.h>
#include <Adafruit_ADS1X15.h>


Adafruit_BMP280 bmp; // I2C
Adafruit_ADS1115 ads;

void setup() {

    Serial.begin(9600);

    unsigned status = bmp.begin();
      if (!status) {

        while (1) {
        Serial.println(F("Could not find a valid BMP280 sensor, check wiring or "
                          "try a different address!"));
        }
    }

    /* Default settings from datasheet. */
    bmp.setSampling(Adafruit_BMP280::MODE_NORMAL,     /* Operating Mode. */
                    Adafruit_BMP280::SAMPLING_X2,     /* Temp. oversampling */
                    Adafruit_BMP280::SAMPLING_X16,    /* Pressure oversampling */
                    Adafruit_BMP280::FILTER_X16,      /* Filtering. */
                    Adafruit_BMP280::STANDBY_MS_500); /* Standby time. */
    
    if (!ads.begin()) {
      while (1) {
      Serial.println("Failed to initialize ADS.");
      }
    }
}

void loop() {
    Serial.print((bmp.readTemperature() * 9/5) + 32); // bmp280_temp_f
    Serial.print(",");

    Serial.print(bmp.readPressure()); // bmp280_press
    Serial.print(",");

    const int analogValue = analogRead(A1); 
    float voltage = analogValue * (3.3 / 1024.0);
    float temperatureC = (voltage - 0.5) * 100.0;
    float temperatureF = (temperatureC * 9.0 / 5.0) + 32.0;

    Serial.print(temperatureF); // tmp36_temp_f
    Serial.print(",");

    Serial.print(ads.readADC_SingleEnded(2));  // Main
    Serial.print(",");

    Serial.print(ads.readADC_SingleEnded(3)); // Amp
    Serial.print(",");

    int usbAnalogValue = analogRead(A3); // USB
    Serial.print(usbAnalogValue);
    Serial.print(",");

    Serial.print(ads.readADC_SingleEnded(0)); // Forward
    Serial.print(",");

    Serial.print(ads.readADC_SingleEnded(1)); // Reverse
    Serial.print(",");

    Serial.println();
    delay(10000);
}
