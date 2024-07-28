//! A cycle-accurate implementation of the NES's 6502-based CPU core.
//!
//! # Modularity
//!
//! A decently large selection of NES emulators structure every component to run
//! under the CPU, essentially letting the CPU drive the whole system. While
//! this is often a simpler approach and takes much less code to implement, it
//! makes the whole system harder to reason about and test. To just run some CPU
//! tests, for example, you also need to run a whole NES system or hack in a
//! pseudo-NES just for testing.
//!
//! Chuck's implementation of the CPU completely decouples it from the rest of
//! the NES, which not only allows for easier refactoring and faster testing, it
//! also allows the CPU emulation to be easily extracted for use elsewhere, in a
//! Commodore 64 or Apple II emulator for example.
//!
//! # Link(s)
//!
//! - <https://www.nesdev.org/6502_cpu.txt>

/// The 6502-based Central Processing Unit (CPU) of the NES.
#[derive(Debug, Clone)]
pub struct Cpu {}
