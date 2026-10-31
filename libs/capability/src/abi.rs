//! The system-call ABI: the message-register calling convention.
//!
//! Invoking a capability carries a packed [`MessageInfo`] word - the method
//! **label**, the number of message-register words used (**length**), and the
//! number of capabilities to transfer (**caps**) - plus the message registers
//! themselves and the invoked capability id. This mirrors seL4's IPC shape and
//! is a natural fit for the message payloads blocking IPC adds in M4
//! ([ADR 0016](../../docs/adr/0016-capability-implementation.md),
//! [ADR 0017](../../docs/adr/0017-threads-and-scheduling.md)).
//!
//! The register assignment is the HAL/boot layer's concern; this module is just
//! the pure packing, so it is host-tested.

/// Number of message-register words carried in registers.
pub const MSG_REGISTERS: usize = 4;

/// A packed message-info word: method `label`, message `length`, and capability
/// transfer `caps` count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MessageInfo(u64);

impl MessageInfo {
    const LABEL_BITS: u64 = 32;
    const LENGTH_SHIFT: u64 = 32;
    const CAPS_SHIFT: u64 = 40;

    /// Build from a `label`, a message `length` (words), and a `caps` count.
    #[must_use]
    pub const fn new(label: u32, length: u8, caps: u8) -> Self {
        Self(
            (label as u64)
                | ((length as u64) << Self::LENGTH_SHIFT)
                | ((caps as u64) << Self::CAPS_SHIFT),
        )
    }

    /// Reinterpret a raw word (as delivered in a register).
    #[must_use]
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    /// The raw word.
    #[must_use]
    pub const fn bits(self) -> u64 {
        self.0
    }

    /// The method label.
    #[must_use]
    pub const fn label(self) -> u32 {
        (self.0 & ((1 << Self::LABEL_BITS) - 1)) as u32
    }

    /// The number of message-register words used (clamped to [`MSG_REGISTERS`]).
    #[must_use]
    pub const fn length(self) -> u8 {
        let n = (self.0 >> Self::LENGTH_SHIFT) as u8;
        if (n as usize) > MSG_REGISTERS {
            MSG_REGISTERS as u8
        } else {
            n
        }
    }

    /// The number of capabilities to transfer with the message.
    #[must_use]
    pub const fn caps(self) -> u8 {
        (self.0 >> Self::CAPS_SHIFT) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_fields() {
        let mi = MessageInfo::new(0xABCD, 3, 1);
        assert_eq!(mi.label(), 0xABCD);
        assert_eq!(mi.length(), 3);
        assert_eq!(mi.caps(), 1);
        assert_eq!(MessageInfo::from_bits(mi.bits()), mi);
    }

    #[test]
    fn length_is_clamped_to_capacity() {
        // A bogus over-long length is clamped, so decoders never read past the
        // register bank.
        let mi = MessageInfo::new(1, 200, 0);
        assert_eq!(mi.length() as usize, MSG_REGISTERS);
    }

    #[test]
    fn fields_are_independent() {
        let mi = MessageInfo::new(u32::MAX, 0, 0);
        assert_eq!(mi.label(), u32::MAX);
        assert_eq!(mi.length(), 0);
        assert_eq!(mi.caps(), 0);
    }
}
