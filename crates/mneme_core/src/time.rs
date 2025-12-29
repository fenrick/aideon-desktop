use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use once_cell::sync::Lazy;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct ValidTime(pub i64);

impl ValidTime {
    pub fn now_micros() -> Self {
        let micros = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as i64;
        Self(micros)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Hlc(pub i64);

// Pack HLC into a portable i64: upper bits = microseconds since epoch, lower bits = counter.
const HLC_COUNTER_BITS: u32 = 12;
const HLC_COUNTER_MASK: u64 = (1u64 << HLC_COUNTER_BITS) - 1;

static LAST_HLC: Lazy<Mutex<u64>> = Lazy::new(|| Mutex::new(0));

impl Hlc {
    pub fn now() -> Self {
        let physical = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64;
        let mut guard = LAST_HLC.lock().expect("HLC mutex poisoned");
        let last = *guard;
        let last_physical = last >> HLC_COUNTER_BITS;
        let last_counter = last & HLC_COUNTER_MASK;
        let (next_physical, next_counter) = if physical > last_physical {
            (physical, 0)
        } else if last_counter < HLC_COUNTER_MASK {
            (last_physical, last_counter + 1)
        } else {
            (last_physical + 1, 0)
        };
        let next = (next_physical << HLC_COUNTER_BITS) | next_counter;
        *guard = next;
        Hlc(next as i64)
    }

    pub fn as_i64(self) -> i64 {
        self.0
    }

    pub fn from_i64(value: i64) -> Self {
        Hlc(value)
    }
}

impl Serialize for Hlc {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i64(self.0)
    }
}

impl<'de> Deserialize<'de> for Hlc {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = i64::deserialize(deserializer)?;
        Ok(Hlc(value))
    }
}
