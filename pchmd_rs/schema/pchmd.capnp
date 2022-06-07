@0xd18ac87e227503c0;

const majorVersion :UInt8 = 0;
const minorVersion :UInt8 = 1;
const patchVersion :UInt8 = 0;

struct Version {
    major @0 :UInt8 = .majorVersion;
    minor @1 :UInt8 = .minorVersion;
    patch @2 :UInt8 = .patchVersion;
}

struct SensorValue {
	union {
		floatValue @0 :Float64;
		boolValue @1 :Bool;
		stringValue @2 :Text;
	}
}

struct SensorData {
    sensorName @0: Text;
    dataSourceName @1: Text;

	current @2 :SensorValue;
    average @3 :SensorValue;
	minimum @4 :SensorValue;
	maximum @5 :SensorValue;

    measurementUnit @6 :MeasurementUnit;
    enum MeasurementUnit {
        none @0;
        volt @1;
        amp @2;
        watt @3;
        joule @4;
        celcius @5;
        second @6;
        rotationPerMinute @7;
        percentage @8;
    }

	isStale @7 :Bool;
}

struct ComputerInfo {
	name @0 :Text; # human readable name for convenience
	uuidUpper @1 :UInt64; # upper 64 bits: typically UUID seeded from mac address
	uuidLower @2 :UInt64; # lower 64 bits: typically UUID seeded from mac address
    operatingSystem @3 :Text;
	serverVersion @4 :Version;
	sensors @5 :List(SensorData);
}
