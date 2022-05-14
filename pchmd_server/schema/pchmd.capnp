@0xd18ac87e227503c0;

struct SensorValue {
	union {
		signedIntegerValue @0 :UInt64;
		unsignedIntegerValue @1 :Int64;
		floatValue @2 :Float64;
	}
}

struct SensorData {
	current @0 :SensorValue;
	min @1 :SensorValue;
	max @2 :SensorValue;
	average @3 :SensorValue;
}

struct PCInfo {
	name @0 :Text; # human readable name for convenience
	uuid @1 :UInt16;
	sensors @2 :List(SensorData);
}

