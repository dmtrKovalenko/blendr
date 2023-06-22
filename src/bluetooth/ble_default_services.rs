use std::collections::HashMap;
use uuid::Uuid;

/// Standard services are always 16bit uuid, while we actually receiving 128bit uuids from ble devices
pub(crate) const fn create_ble_uuid(uuid: u16) -> Uuid {
    let base_uuid: u128 = 0x0000_0000_0000_1000_8000_0080_5F9B_34FB;
    let converted_uuid = ((uuid as u128) << 96) | base_uuid;

    Uuid::from_u128(converted_uuid)
}

lazy_static::lazy_static! {
    pub static ref SPECIAL_SERVICES_NAMES: HashMap<uuid::Uuid, &'static str> = HashMap::from([
        (create_ble_uuid(0x1800), "Generic Access (0x1800)"),
        (create_ble_uuid(0x1801), "Generic Attribute (0x1800)"),
        (create_ble_uuid(0x1802), "Immediate Alert (0x1800)"),
        (create_ble_uuid(0x1803), "Link Loss (0x1800)"),
        (create_ble_uuid(0x1804), "Tx Power (0x1800)"),
        (create_ble_uuid(0x1805), "Current Time Service (0x1800)"),
        (create_ble_uuid(0x1806), "Reference Time Update Service (0x1800)"),
        (create_ble_uuid(0x1807), "Next DST Change Service (0x1800)"),
        (create_ble_uuid(0x1808), "Glucose (0x1800)"),
        (create_ble_uuid(0x1809), "Health Thermometer (0x1800)"),
        (create_ble_uuid(0x180A), "Device Information (0x1800)"),
        (create_ble_uuid(0x180D), "Heart Rate (0x1800)"),
        (create_ble_uuid(0x180E), "Phone Alert Status Service (0x1800)"),
        (create_ble_uuid(0x180F), "Battery (0x1800)"),
        (create_ble_uuid(0x1810), "Blood Pressure (0x1800)"),
        (create_ble_uuid(0x1811), "Alert Notification Service (0x1800)"),
        (create_ble_uuid(0x1812), "Human Interface Device (0x1800)"),
        (create_ble_uuid(0x1813), "Scan Parameters (0x1800)"),
        (create_ble_uuid(0x1814), "Running Speed and Cadence (0x1800)"),
        (create_ble_uuid(0x1815), "Cycling Speed and Cadence (0x1800)"),
        (create_ble_uuid(0x1816), "Cycling Power (0x1800)"),
        (create_ble_uuid(0x1818), "Cycling Power Vector (0x1800)"),
        (create_ble_uuid(0x1819), "Location and Navigation (0x1800)"),
        (create_ble_uuid(0x181A), "Environmental Sensing (0x1800)"),
        (create_ble_uuid(0x181B), "Body Composition (0x1800)"),
        (create_ble_uuid(0x181C), "User Data (0x1800)"),
        (create_ble_uuid(0x181D), "Weight Scale (0x1800)"),
        (create_ble_uuid(0x181E), "Bond Management (0x1800)"),
        (create_ble_uuid(0x181F), "Continuous Glucose Monitoring (0x1800)"),
        (create_ble_uuid(0x1820), "Internet Protocol Support Service (0x1800)"),
        (create_ble_uuid(0x1821), "Indoor Positioning (0x1800)"),
        (create_ble_uuid(0x1822), "Pulse Oximeter (0x1800)"),
        (create_ble_uuid(0x1823), "HTTP Proxy (0x1800)"),
        (create_ble_uuid(0x1824), "Transport Discovery (0x1800)"),
        (create_ble_uuid(0x1825), "Object Transfer (0x1800)"),
    ]);

    pub static ref SPECIAL_CHARACTERISTICS_NAMES: HashMap<uuid::Uuid, &'static str> = HashMap::from([
      // Generic Access Service (0x1800)
      (create_ble_uuid(0x2A00), "Device Name (0x2A00)"),
      (create_ble_uuid(0x2A01), "Appearance (0x2A01)"),
      (create_ble_uuid(0x2A02), "Peripheral Privacy Flag (0x2A02)"),
      (create_ble_uuid(0x2A03), "Reconnection Address (0x2A03)"),
      (create_ble_uuid(0x2A04), "Peripheral Preferred Connection Parameters (0x2A04)"),

      // Generic Attribute Service (0x1801)
      (create_ble_uuid(0x2A05), "Service Changed (0x2A05)"),

      // Immediate Alert Service (0x1802)
      (create_ble_uuid(0x2A06), "Alert Level (0x2A06)"),

      // Link Loss Service (0x1803)
      (create_ble_uuid(0x2A06), "Alert Level (0x2A06)"),

      // TX Power Service (0x1804)
      (create_ble_uuid(0x2A07), "Tx Power Level (0x2A07)"),
      // Current Time Service (0x1805)
      (create_ble_uuid(0x2A0B), "Current Time (0x2A0B)"),
      (create_ble_uuid(0x2A0C), "Local Time Information (0x2A0C)"),
      (create_ble_uuid(0x2A0D), "Reference Time Information (0x2A0D)"),

      // Battery Service (0x180F)
      (create_ble_uuid(0x2A19), "Battery Level (0x2A19)"),

      // Reference Time Update Service (0x1806)
      (create_ble_uuid(0x2A16), "Time Update Control Point (0x2A16)"),
      (create_ble_uuid(0x2A17), "Time Update State (0x2A17)"),

      // Next DST Change Service (0x1807)
      (create_ble_uuid(0x2A1D), "Time with DST (0x2A1D)"),

      // Glucose Service (0x1808)
      (create_ble_uuid(0x2A18), "Glucose Measurement (0x2A18)"),
      (create_ble_uuid(0x2A34), "Glucose Feature (0x2A34)"),
      (create_ble_uuid(0x2A51), "Glucose Measurement Context (0x2A51)"),

      // Health Thermometer Service (0x1809)
      (create_ble_uuid(0x2A1C), "Temperature Measurement (0x2A1C)"),
      (create_ble_uuid(0x2A1D), "Temperature Type (0x2A1D)"),

      // Device Information Service (0x180A)
      (create_ble_uuid(0x2A29), "Manufacturer Name String (0x2A29)"),
      (create_ble_uuid(0x2A24), "Model Number String (0x2A24)"),
      (create_ble_uuid(0x2A25), "Serial Number String (0x2A25)"),
      (create_ble_uuid(0x2A26), "Firmware Revision String (0x2A26)"),
      (create_ble_uuid(0x2A27), "Hardware Revision String (0x2A27)"),
      (create_ble_uuid(0x2A28), "Software Revision String (0x2A28)"),
      (create_ble_uuid(0x2A23), "System ID (0x2A23)"),
      (create_ble_uuid(0x2A2A), "IEEE 11073-20601 Regulatory Certification Data List (0x2A2A)"),
      (create_ble_uuid(0x2A50), "PnP ID (0x2A50)"),

      // Heart Rate Service (0x180D)
      (create_ble_uuid(0x2A37), "Heart Rate Measurement (0x2A37)"),
      (create_ble_uuid(0x2A38), "Body Sensor Location (0x2A38)"),
      (create_ble_uuid(0x2A39), "Heart Rate Control Point (0x2A39)"),

      // Phone Alert Status Service (0x180E)
      (create_ble_uuid(0x2A3F), "Alert Status (0x2A3F)"),
      (create_ble_uuid(0x2A40), "Ringer Control Point (0x2A40)"),
      (create_ble_uuid(0x2A41), "Ringer Setting (0x2A41)"),
      (create_ble_uuid(0x2A42), "Alert Category ID Bit Mask (0x2A42)"),
      (create_ble_uuid(0x2A43), "Alert Category ID (0x2A43)"),
      (create_ble_uuid(0x2A44), "Alert Notification Control Point (0x2A44)"),
      (create_ble_uuid(0x2A45), "Unread Alert Status (0x2A45)"),
      (create_ble_uuid(0x2A46), "New Alert (0x2A46)"),
      (create_ble_uuid(0x2A47), "Supported New Alert Category (0x2A47)"),
      (create_ble_uuid(0x2A48), "Supported Unread Alert Category (0x2A48)"),
      (create_ble_uuid(0x2A49), "Blood Pressure Feature (0x2A49)"),
      (create_ble_uuid(0x2A50), "PnP ID (0x2A50)"),

      // Running Speed and Cadence Service (0x1814)
      (create_ble_uuid(0x2A53), "RSC Measurement (0x2A53)"),
      (create_ble_uuid(0x2A54), "RSC Feature (0x2A54)"),
      (create_ble_uuid(0x2A55), "Sensor Location (0x2A55)"),
      (create_ble_uuid(0x2A5D), "SC Control Point (0x2A5D)"),

      // Cycling Speed and Cadence Service (0x1816)
      (create_ble_uuid(0x2A5B), "CSC Measurement (0x2A5B)"),
      (create_ble_uuid(0x2A5C), "CSC Feature (0x2A5C)"),

      // Environmental Sensing Service (0x181A)
      (create_ble_uuid(0x2A6E), "Temperature (0x2A6E)"),
      (create_ble_uuid(0x2A6F), "Humidity (0x2A6F)"),
      (create_ble_uuid(0x2A76), "Irradiance (0x2A76)"),
      (create_ble_uuid(0x2A77), "Rainfall (0x2A77)"),
      (create_ble_uuid(0x2A78), "Wind Speed (0x2A78)"),
      (create_ble_uuid(0x2A79), "Barometric Pressure Trend (0x2A79)"),
      (create_ble_uuid(0x2A7A), "Magnetic Declination (0x2A7A)"),
      (create_ble_uuid(0x2A6D), "Pressure (0x2A6D)"),

      // Body Composition Service (0x181B)
      (create_ble_uuid(0x2A9C), "Body Composition Measurement (0x2A9C)"),
      (create_ble_uuid(0x2A9D), "Body Composition Feature (0x2A9D)"),
      (create_ble_uuid(0x2A9E), "Body Composition Control Point (0x2A9E)"),

      // User Data Service (0x181C)
      (create_ble_uuid(0x2A9F), "User Control Point (0x2A9F)"),
      (create_ble_uuid(0x2AA0), "User Status (0x2AA0)"),
      (create_ble_uuid(0x2AA1), "Heart Rate Max (0x2AA1)"),
      (create_ble_uuid(0x2AA2), "Resting Heart Rate (0x2AA2)"),
      (create_ble_uuid(0x2AA3), "Maximum Recommended Heart Rate (0x2AA3)"),
      (create_ble_uuid(0x2AA4), "Aerobic Threshold (0x2AA4)"),
      (create_ble_uuid(0x2AA5), "Anaerobic Threshold (0x2AA5)"),
      (create_ble_uuid(0x2AA6), "Sport Type for Aerobic and Anaerobic Thresholds (0x2AA6)"),
      (create_ble_uuid(0x2AA7), "Fat Burn Heart Rate Lower Limit (0x2AA7)"),
      (create_ble_uuid(0x2AA8), "Fat Burn Heart Rate Upper Limit (0x2AA8)"),
      (create_ble_uuid(0x2AA9), "Aerobic Heart Rate Lower Limit (0x2AA9)"),
      (create_ble_uuid(0x2AAA), "Aerobic Heart Rate Upper Limit (0x2AAA)"),
      (create_ble_uuid(0x2AAB), "Anaerobic Heart Rate Lower Limit (0x2AAB)"),
      (create_ble_uuid(0x2AAC), "Anaerobic Heart Rate Upper Limit (0x2AAC)"),
      (create_ble_uuid(0x2AAD), "Five Zone Heart Rate Limits (0x2AAD)"),
      (create_ble_uuid(0x2AAE), "Three Zone Heart Rate Limits (0x2AAE)"),
      (create_ble_uuid(0x2AAF), "Two Zone Heart Rate Limit (0x2AAF)"),

      // Weight Scale Service (0x181D)
      (create_ble_uuid(0x2A9B), "Weight Measurement (0x2A9B)"),
      (create_ble_uuid(0x2A9D), "Body Composition Feature (0x2A9D)"),
      (create_ble_uuid(0x2A9E), "Body Composition Control Point (0x2A9E)"),

      // Bond Management Service (0x181E)
      (create_ble_uuid(0x2AA1), "Bond Management Control Point (0x2AA1)"),
      (create_ble_uuid(0x2AA2), "Bond Management Feature (0x2AA2)"),

      // Continuous Glucose Monitoring Service (0x181F)
      (create_ble_uuid(0x2AA7), "CGM Measurement (0x2AA7)"),
      (create_ble_uuid(0x2AA8), "CGM Feature (0x2AA8)"),
      (create_ble_uuid(0x2AA9), "CGM Status (0x2AA9)"),
      (create_ble_uuid(0x2AAA), "CGM Session Start Time (0x2AAA)"),
      (create_ble_uuid(0x2AAB), "CGM Session Run Time (0x2AAB)"),
      (create_ble_uuid(0x2AAC), "CGM Specific Ops Control Point (0x2AAC)"),
      (create_ble_uuid(0x2AAD), "Indoor Positioning Configuration (0x2AAD)"),
      (create_ble_uuid(0x2AAE), "Latitude (0x2AAE)"),
      (create_ble_uuid(0x2AAF), "Longitude (0x2AAF)"),
      (create_ble_uuid(0x2AB0), "Local North Coordinate (0x2AB0)"),
      (create_ble_uuid(0x2AB1), "Local East Coordinate (0x2AB1)"),
      (create_ble_uuid(0x2AB2), "Floor Number (0x2AB2)"),
      (create_ble_uuid(0x2AB3), "Altitude (0x2AB3)"),
    (create_ble_uuid(0x2AB4), "Uncertainty (0x2AB4)"),
  ]);
}

#[test]
pub(crate) fn resolves_special_services() {
    assert_eq!(
        SPECIAL_SERVICES_NAMES.get(&Uuid::from_u128(0x0000180a_0000_1000_8000_00805f9b34fb)),
        Some(&"Device Information")
    );
}
