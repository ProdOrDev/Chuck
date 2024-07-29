//! A cycle-accurate implementation of the NES's 6502-based CPU core.
//!
//! # Modularity
//!
//! This 6502 implementation is completely decoupled from the rest of Chuck's
//! implementation, which not only allows for easier refactoring and a faster
//! testing process, it also allows this CPU core to be easily transplanted
//! into another emulator, like a Commodore 64 or an Apple II for example.
//!
//! By Chuck having this modularity, it bypasses a minor (or major) issue that
//! exists in other (beginner level) NES emulators. To run/test the CPU in
//! isolation you need a whole NES subsystem attached or you need to hackily
//! implement a pseudo-NES just for testing. Additionally, Chuck's approach is
//! simpler, takes less code to implement, and overall makes the whole system
//! easier to reason about.
//!
//! # Link(s)
//!
//! - <https://www.nesdev.org/6502_cpu.txt>

bitflags::bitflags! {
    /// The status flags of a 6502 CPU.
    ///
    /// ```text
    /// 7  bit  0
    /// ---- ----
    /// NV1B DIZC
    /// |||| ||||
    /// |||| |||+- Carry
    /// |||| ||+-- Zero
    /// |||| |+--- Interrupt Disable
    /// |||| +---- Decimal
    /// |||+------ (No CPU effect; the B flag)
    /// ||+------- (No CPU effect; always pushed as 1)
    /// |+-------- Overflow
    /// +--------- Negative
    /// ```
    ///
    /// # Link(s)
    ///
    /// - <https://www.nesdev.org/wiki/Status_flags>
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Flags: u8 {
        /// The carry flag, `C`.
        const C = 1 << 0;
        /// The zero flag, `Z`.
        const Z = 1 << 1;
        /// The interrupt-disable flag, `I`.
        const I = 1 << 2;
        /// The decimal-mode flag, `D`.
        const D = 1 << 3;
        /// The overflow flag, `V`.
        const V = 1 << 6;
        /// The negative flag, `N`.
        const N = 1 << 7;
    }
}

bitflags::bitflags! {
    /// The I/O control pins of a 6502 CPU.
    ///
    /// # Link(s)
    ///
    /// - <https://www.nesdev.org/wiki/IRQ>
    /// - <https://www.nesdev.org/wiki/NMI>
    /// - <https://www.nesdev.org/wiki/CPU_pinout>
    /// - <http://user.xmission.com/~trevin/atari/6502_pinout.html>
    #[derive(Debug, Clone, Copy)]
    pub struct Pins: u8 {
        /// The synchronize-output pin, `SYNC`.
        ///
        /// This pin is set and used to identify the cycles when the CPU is
        /// fetching an opcode byte from memory.
        const SYNC = 1 << 0;
        /// The interrupt-request pin, `IRQ`.
        ///
        /// This pin can be set externally to trigger a maskable (ignorable)
        /// interrupt to user-defined code in the CPU.
        const IRQ = 1 << 1;
        /// The non-maskable-interrupt pin, `NMI`.
        ///
        /// This pin can be set externally to trigger a non-maskable (non
        /// ignoreable) interrupt to user-defined code in the CPU.
        const NMI = 1 << 2;
        /// The ready-input pin, `RDY`.
        ///
        /// This pin can be set externally to single-step the CPU or wait for
        /// slow memory to load. When set, this pin will stall the CPU's
        /// execution of `read` cycles until the pin is disabled again.
        const RDY = 1 << 3;
    }
}

/// The memory bus of a 6502 CPU.
///
/// # Link(s)
///
/// - <https://www.nesdev.org/wiki/CPU_pinout>
/// - <http://user.xmission.com/~trevin/atari/6502_pinout.html>
#[derive(Debug, Clone)]
pub struct Bus {
    /// The 16-bit address bus.
    ///
    /// This corresponds to the pins labeled `A0`-`A15` on a 6502.
    pub addr: u16,
    /// The 8-bit data bus.
    ///
    /// This corresponds to the pins labeled `D0`-`D7` on a 6502.
    pub data: u8,
    /// The memory access state of the bus.
    ///
    /// This corresponds to the pin labeled `R/W` on a 6502.
    pub write: bool,
}

/// The registers of a 6502 CPU.
///
/// # Link(s)
///
/// - <https://www.nesdev.org/wiki/CPU_registers>
#[derive(Debug, Clone)]
pub struct Registers {
    /// The flags register, `P`.
    pub flags: Flags,
    /// The general purpose accumulator, `A`.
    pub a: u8,
    /// The first index register, `X`.
    pub x: u8,
    /// The second index register, `Y`.
    pub y: u8,
    /// The stack pointer, `S`.
    pub sp: u8,
    /// The program counter, `PC`.
    pub pc: u16,
}

/// The 6502-based Central Processing Unit (CPU) of the NES.
#[derive(Debug, Clone)]
pub struct Cpu {
    /// The I/O control pins of the CPU.
    pub pins: Pins,
    /// The memory bus of the CPU.
    pub bus: Bus,
    /// The registers of the CPU.
    pub regs: Registers,
}
