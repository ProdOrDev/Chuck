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
//! easier to reason about and test.
//!
//! # Link(s)
//!
//! - <https://www.nesdev.org/wiki/CPU>
//! - <https://www.nesdev.org/6502_cpu.txt>

bitflags::bitflags! {
    /// The status flags of a 6502 CPU.
    ///
    /// ```no-run
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
        ///
        /// This is the pin used internally to implement Direct Memory Access
        /// (DMA) on the NES.
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

/// The partial instruction states of the CPU.
///
/// # Link(s)
///
/// - <https://www.nesdev.org/wiki/Visual6502wiki/6502_State_Machine>
#[derive(Debug, Clone, Copy)]
#[rustfmt::skip]
#[repr(u8)]
pub(crate) enum State {
    T0, T1, T2, T3, T4, T5, T6, T7
}

/// A Timing Control Unit (TCU) for CPU instructions.
///
/// An vanilla hardware 6502 does not actually have a dedicated TCU, instead it
/// has a collection of PLA inputs `T0`-`T5` and other inputs like `VEC0`/`VEC1`
/// which combine to run the CPU. This specific design of TCU comes from the
/// more modern WDC 65C02S, which is a dedicated unit that provides the timing
/// for each instruction cycle that is executed within the CPU.
///
/// # Link(s)
///
/// - <https://www.westerndesigncenter.com/wdc/documentation/w65c02s.pdf>
#[derive(Debug, Clone)]
pub(crate) struct Tcu {
    /// The current partial instruction state.
    pub(crate) state: State,
}

impl Tcu {
    /// Goto the next partial instruction state.
    pub(crate) fn advance(&mut self) {
        self.state = match self.state {
            State::T0 => State::T1,
            State::T1 => State::T2,
            State::T2 => State::T3,
            State::T3 => State::T4,
            State::T4 => State::T5,
            State::T5 => State::T6,
            State::T6 => State::T7,
            State::T7 => State::T0,
        }
    }

    /// Reset the current partial instruction state.
    pub(crate) fn reset(&mut self) {
        self.state = State::T7;
    }
}

/// A timing pipeline for controlling when interrupts are serviced.
///
/// When an external interrupt request pin, such as `IRQ` or `NMI`, is pulled
/// active, it will be placed onto a (emulator fictional) timing pipeline. Every
/// CPU cycle, the pipeline will shift its internal data to the left. Once the
/// data contained in the pipeline actives any bit specified inside a given
/// bitmask, the interrupt denoted by this pipeline is viable to be serviced.
///
/// There can be different bitmasks specified because `IRQ` interrupts vary from
/// `NMI` interrupts in the timeline in which they can be serviced.
///
/// # Link(s)
///
/// - <https://www.nesdev.org/wiki/IRQ>
/// - <https://www.nesdev.org/wiki/NMI>
/// - <https://www.nesdev.org/wiki/CPU_interrupts>
/// - <https://www.nesdev.org/wiki/Visual6502wiki/6502_Interrupt_Recognition_Stages_and_Tolerances>
#[derive(Debug, Clone)]
pub(crate) struct Pipeline<const MASK: u16> {
    data: u16,
}

impl<const MASK: u16> Pipeline<MASK> {
    /// Register an interrupt request on this pipeline if its corresponding pin
    /// is pulled active.
    pub(crate) fn register_with(&mut self, pin: bool) {
        if pin {
            self.data |= 0x100;
        }
    }

    /// Check if the interrupt denoted by this pipeline can be serviced.
    #[must_use]
    pub(crate) fn is_serviceable(&self) -> bool {
        self.data & MASK != 0
    }

    /// Trim this pipeline.
    ///
    /// This prevents the CPU from accidentally re-servicing the same interrupt
    /// request.
    pub(crate) fn trim(&mut self) {
        self.data &= 0x3ff;
    }

    /// Shift the data in this pipeline.
    pub(crate) fn shift(&mut self) {
        self.data <<= 1;
    }

    /// Undo a pipeline data shift.
    pub(crate) fn undo(&mut self) {
        self.data >>= 1;
    }
}

/// The type of interrupts that can be serviced by the CPU.
///
/// # Link(s)
///
/// - <https://www.nesdev.org/wiki/CPU_interrupts>
#[derive(Debug, Clone, Copy)]
pub(crate) enum Interrupt {
    /// A software requested break interrupt.
    Brk,
    /// An externally requested maskable interrupt.
    Irq,
    /// An externally requested non-maskable interrupt.
    Nmi,
    /// An externally requested reset interrupt.
    Res,
}

/// The opcode value of the `BRK` instruction.
const BRK: u8 = 0x00;

/// The 6502-based Central Processing Unit (CPU) of the NES.
#[derive(Debug, Clone)]
pub struct Cpu {
    /// The I/O control pins.
    pub pins: Pins,
    /// The memory bus.
    pub bus: Bus,
    /// The registers.
    pub regs: Registers,

    /// A flag denoting if the CPU is jammed.
    pub(crate) jammed: bool,

    /// The next interrupt type that will be serviced by the CPU.
    ///
    /// This value only takes effect when a `BRK` instruction is executed, it
    /// is not polled to determine if the CPU should service an interrupt.
    pub(crate) schedule: Interrupt,
    /// A copy of the previous `NMI` pin value used for edge detection.
    pub(crate) nmi_edge: bool,
    /// The timing pipeline for `IRQ` interrupts.
    pub(crate) irq_pip: Pipeline<0x0400>,
    /// The timing pipeline for `NMI` interrupts.
    pub(crate) nmi_pip: Pipeline<0xfc00>,

    /// The internal Address Decoding Latch (ADL).
    ///
    /// This is used to calculate the effective address of instructions and to
    /// store data for these instructions over multiple cycles.
    pub(crate) adl: u16,
    /// The opcode that is currently being executed.
    pub(crate) opcode: u8,
    /// The Timing Control Unit (TCU).
    pub(crate) tcu: Tcu,
}

impl Cpu {
    /// Create a new CPU.
    #[must_use]
    pub fn new() -> Self {
        Self {
            pins: Pins::empty(),
            bus: Bus {
                addr: 0,
                data: 0,
                write: false,
            },
            regs: Registers {
                flags: Flags::empty(),
                a: 0,
                x: 0,
                y: 0,
                sp: 0,
                pc: 0,
            },
            jammed: false,
            schedule: Interrupt::Res,
            nmi_edge: false,
            irq_pip: Pipeline { data: 0 },
            nmi_pip: Pipeline { data: 0 },
            adl: 0,
            opcode: BRK,
            tcu: Tcu { state: State::T7 },
        }
    }
}
