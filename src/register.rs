use core::convert::TryInto;
use num_enum::TryFromPrimitive;

/// Possible I²C slave addresses.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum SlaveAddr {
    /// Default slave address (`0x18`)
    Default = 0x18,

    /// Alternate slave address (`0x19`)
    Alternate = 0x19,
}

impl SlaveAddr {
    pub fn addr(self) -> u8 {
        self as u8
    }
}

/// Enumerate all device registers.
#[allow(dead_code, non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Register {
    CTRL1 = 0x20,
    CTRL2 = 0x21,
    CTRL3 = 0x22,
    CTRL4 = 0x23,
    CTRL5 = 0x24,
    HP_FILTER_RESET = 0x25,
    REFERENCE = 0x26,
    STATUS = 0x27,
    OUT_X_L = 0x28,
    OUT_X_H = 0x29,
    OUT_Y_L = 0x2A,
    OUT_Y_H = 0x2B,
    OUT_Z_L = 0x2C,
    OUT_Z_H = 0x2D,
    INT1_CFG = 0x30,
    INT1_SRC = 0x31,
    INT1_THS = 0x32,
    INT1_DURATION = 0x33,
    INT2_CFG = 0x34,
    INT2_SRC = 0x35,
    INT2_THS = 0x36,
    INT2_DURATION = 0x37,
}

impl Register {
    /// Get register address
    pub fn addr(self) -> u8 {
        self as u8
    }

    /// Is the register read-only?
    pub fn read_only(self) -> bool {
        matches!(
            self,
            Register::STATUS
                | Register::OUT_X_L
                | Register::OUT_X_H
                | Register::OUT_Y_L
                | Register::OUT_Y_H
                | Register::OUT_Z_L
                | Register::OUT_Z_H
                | Register::INT1_SRC
                | Register::INT2_SRC
        )
    }
}

/// Full-scale selection.
#[allow(non_camel_case_types)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub enum Range {
    /// ±24g
    G24 = 0b11,

    /// ±12g
    G12 = 0b01,

    /// ±6g (Default)
    G6 = 0b00,
}

impl Range {
    pub const fn bits(self) -> u8 {
        self as u8
    }

    /// Convert the range into an value in mili-g
    pub const fn as_mg(self) -> u8 {
        match self {
            // XXX this is wrong and I don't know why
            Range::G24 => 192,
            Range::G12 => 96,
            Range::G6 => 48,
        }
    }
}

impl Default for Range {
    fn default() -> Self {
        Range::G6
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub struct Threshold(pub(crate) u8);

impl Threshold {
    /// Convert a value in multiples of the `g` constant (roughly 9.81) to a threshold.
    ///
    ///     assert_eq!(Threshold::g(Range::G2, 1.1), 69);
    #[inline(always)]
    pub fn g(range: Range, gs: f32) -> Self {
        Self::mg(range, gs * 1000.0)
    }

    #[inline(always)]
    pub fn mg(range: Range, mgs: f32) -> Self {
        let value = mgs / (range.as_mg() as f32);

        let result = crude_ceil(value);

        Threshold(result.try_into().unwrap())
    }

    pub const ZERO: Self = Threshold(0);
}

/// a crude `.ceil()`, the std one is not currently available when using no_std
fn crude_ceil(value: f32) -> u64 {
    let truncated = value as u64;

    let round_up = value - (truncated as f32) > 0.0;

    if round_up {
        truncated + 1
    } else {
        truncated
    }
}

/// Output data rate.
#[allow(non_camel_case_types)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub enum DataRate {
    /// 1000Hz
    Hz_1000 = 0b00111,

    /// 400Hz
    Hz_400 = 0b00110,

    /// 100Hz
    Hz_100 = 0b00101,

    /// 50Hz
    Hz_50 = 0b00100,

    /// 10Hz
    Hz_10 = 0b11000,

    /// 5Hz
    Hz_5 = 0b10100,

    /// 2Hz
    Hz_2 = 0b10000,

    /// 1Hz
    Hz_1 = 0b01100,

    /// 0.5Hz
    Hz_05 = 0b01000,

    /// Power down
    PowerDown = 0b00000,
}

impl DataRate {
    pub const fn bits(self) -> u8 {
        self as u8
    }

    pub const fn sample_rate(self) -> f32 {
        match self {
            DataRate::Hz_1000 => 1000.0,
            DataRate::Hz_400 => 400.0,
            DataRate::Hz_100 => 100.0,
            DataRate::Hz_50 => 50.0,
            DataRate::Hz_10 => 10.0,
            DataRate::Hz_5 => 5.0,
            DataRate::Hz_2 => 2.0,
            DataRate::Hz_1 => 1.0,
            DataRate::Hz_05 => 0.5,
            DataRate::PowerDown => 0.0,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub struct Duration(pub(crate) u8);

impl Duration {
    /// Convert a number of seconds into a duration. Internally a duration is represented
    /// as a multiple of `1 / ODR` where ODR (the output data rate) is of type [`DataRate`].
    #[inline(always)]
    pub fn seconds(output_data_rate: DataRate, seconds: f32) -> Self {
        let duration = output_data_rate.sample_rate() * seconds;

        Self(duration as u8)
    }

    /// Convert a number of miliseconds into a duration. Internally a duration is represented
    /// as a multiple of `1 / ODR` where ODR (the output data rate) is of type [`DataRate`].
    ///
    ///     assert_eq!(Duration::miliseconds(DataRate::Hz_400, 25.0), 10);
    #[inline(always)]
    pub fn miliseconds(output_data_rate: DataRate, miliseconds: f32) -> Self {
        Self::seconds(output_data_rate, miliseconds * 1000.0)
    }

    pub const ZERO: Self = Duration(0);
}

/// Data status structure. Decoded from the `STATUS_REG` register.
///
/// `STATUS_REG` has the following bit fields:
///   * `ZYXOR` - X, Y and Z-axis data overrun
///   * `ZOR` - Z-axis data overrun
///   * `YOR` - Y-axis data overrun
///   * `XOR` - X-axis data overrun
///   * `ZYXDA` - X, Y and Z-axis new data available
///   * `ZDA` - Z-axis new data available
///   * `YDA` Y-axis new data available
///   * `XDA` X-axis new data available
///
/// This struct splits the fields into more convenient groups:
///  * `zyxor` -> `ZYXOR`
///  * `xyzor` -> (`XOR`, `YOR`, `ZOR`)
///  * `zyxda` -> `ZYXDA`
///  * `xyzda` -> (`XDA`, `YDA`, `ZDA`)
#[derive(Debug)]
pub struct DataStatus {
    /// ZYXOR bit
    pub zyxor: bool,

    /// (XOR, YOR, ZOR) bits
    pub xyzor: (bool, bool, bool),

    /// ZYXDA bit
    pub zyxda: bool,

    /// (XDA, YDA, ZDA) bits
    pub xyzda: (bool, bool, bool),
}

// === CTRL_REG1 (20h) ===

pub const ODR_MASK: u8 = 0b1111_0000;
pub const LP_EN: u8 = 0b0000_1000;
pub const Z_EN: u8 = 0b0000_0100;
pub const Y_EN: u8 = 0b0000_0010;
pub const X_EN: u8 = 0b0000_0001;

// === CTRL_REG4 (23h) ===

pub const BDU: u8 = 0b1000_0000;
pub const FS_MASK: u8 = 0b0011_0000;
pub const HR: u8 = 0b0000_1000;

// === STATUS_REG (27h) ===

pub const ZYXOR: u8 = 0b1000_0000;
pub const ZOR: u8 = 0b0100_0000;
pub const YOR: u8 = 0b0010_0000;
pub const XOR: u8 = 0b0001_0000;
pub const ZYXDA: u8 = 0b0000_1000;
pub const ZDA: u8 = 0b0000_0100;
pub const YDA: u8 = 0b0000_0010;
pub const XDA: u8 = 0b0000_0001;

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn threshold_g_vs_mg() {
        assert_eq!(
            Threshold::g(Range::G2, 1.5),
            Threshold::mg(Range::G2, 1500.0)
        );
    }

    #[test]
    fn duration_seconds_vs_miliseconds() {
        assert_eq!(
            Duration::seconds(DataRate::Hz_400, 1.5),
            Duration::miliseconds(DataRate::Hz_400, 1500.0)
        );
    }
}
