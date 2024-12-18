// SPDX-FileCopyrightText: 2023-2024 TriliTech <contact@trili.tech>
//
// SPDX-License-Identifier: MIT

use crate::{
    default::ConstDefault,
    machine_state::{self, bus::main_memory},
    pvm::sbi,
    state_backend::{self, Atom, Cell},
    traps::EnvironException,
};
use std::{
    convert::Infallible,
    fmt,
    io::{stdout, Write},
    ops::Bound,
};

/// PVM configuration
pub struct PvmHooks<'a> {
    pub putchar_hook: Box<dyn FnMut(u8) + 'a>,
}

impl<'a> PvmHooks<'a> {
    /// Create a new configuration.
    pub fn new<F: FnMut(u8) + 'a>(putchar: F) -> Self {
        Self {
            putchar_hook: Box::new(putchar),
        }
    }
}

impl PvmHooks<'static> {
    /// Hook that does nothing.
    pub fn none() -> Self {
        Self {
            putchar_hook: Box::new(|_| {}),
        }
    }
}

/// The default PVM configuration prints all debug information from the kernel
/// to the standard output.
impl<'a> Default for PvmHooks<'a> {
    fn default() -> Self {
        fn putchar(char: u8) {
            stdout().lock().write_all(&[char]).unwrap();
        }

        Self::new(putchar)
    }
}

/// PVM state layout
pub type PvmLayout<ML, CL> = (
    state_backend::Atom<u64>,
    machine_state::MachineStateLayout<ML, CL>,
    Atom<PvmStatus>,
);

/// PVM status
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
    strum::EnumCount,
)]
#[repr(u8)]
pub enum PvmStatus {
    Evaluating,
    WaitingForInput,
    WaitingForMetadata,
}

impl ConstDefault for PvmStatus {
    const DEFAULT: Self = Self::Evaluating;
}

impl fmt::Display for PvmStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status = match self {
            PvmStatus::Evaluating => "Evaluating",
            PvmStatus::WaitingForInput => "Waiting for input message",
            PvmStatus::WaitingForMetadata => "Waiting for metadata",
        };
        f.write_str(status)
    }
}

/// Value for the initial version
const INITIAL_VERSION: u64 = 0;

/// Proof-generating virtual machine
pub struct Pvm<
    ML: main_memory::MainMemoryLayout,
    CL: machine_state::CacheLayouts,
    M: state_backend::ManagerBase,
> {
    version: state_backend::Cell<u64, M>,
    pub(crate) machine_state: machine_state::MachineState<ML, CL, M>,
    status: Cell<PvmStatus, M>,
}

impl<
        ML: main_memory::MainMemoryLayout,
        CL: machine_state::CacheLayouts,
        M: state_backend::ManagerBase,
    > Pvm<ML, CL, M>
{
    /// Bind the PVM to the given allocated region.
    pub fn bind(space: state_backend::AllocatedOf<PvmLayout<ML, CL>, M>) -> Self {
        Self {
            version: space.0,
            machine_state: machine_state::MachineState::bind(space.1),
            status: space.2,
        }
    }

    /// Given a manager morphism `f : &M -> N`, return the layout's allocated structure containing
    /// the constituents of `N` that were produced from the constituents of `&M`.
    pub fn struct_ref<'a, F: state_backend::FnManager<state_backend::Ref<'a, M>>>(
        &'a self,
    ) -> state_backend::AllocatedOf<PvmLayout<ML, CL>, F::Output> {
        (
            self.version.struct_ref::<F>(),
            self.machine_state.struct_ref::<F>(),
            self.status.struct_ref::<F>(),
        )
    }

    /// Reset the PVM state.
    pub fn reset(&mut self)
    where
        M: state_backend::ManagerReadWrite,
    {
        self.version.write(INITIAL_VERSION);
        self.machine_state.reset();
        self.status.write(PvmStatus::DEFAULT);
    }

    /// Handle an exception using the defined Execution Environment.
    pub fn handle_exception(
        &mut self,
        hooks: &mut PvmHooks<'_>,
        exception: EnvironException,
    ) -> bool
    where
        M: state_backend::ManagerReadWrite,
    {
        sbi::handle_call(&mut self.status, &mut self.machine_state, hooks, exception)
    }

    /// Perform one evaluation step.
    pub fn eval_one(&mut self, hooks: &mut PvmHooks<'_>)
    where
        M: state_backend::ManagerReadWrite,
    {
        if let Err(exc) = self.machine_state.step() {
            self.handle_exception(hooks, exc);
        }
    }

    /// Perform a range of evaluation steps. Returns the actual number of steps
    /// performed.
    ///
    /// If an environment trap is raised, handle it and
    /// return the number of retired instructions until the raised trap
    ///
    /// NOTE: instructions which raise exceptions / are interrupted are NOT retired
    ///       See section 3.3.1 for context on retired instructions.
    /// e.g: a load instruction raises an exception but the first instruction
    /// of the trap handler will be executed and retired,
    /// so in the end the load instruction which does not bubble it's exception up to
    /// the execution environment will still retire an instruction, just not itself.
    /// (a possible case: the privilege mode access violation is treated in EE,
    /// but a page fault is not)
    pub fn eval_max(&mut self, hooks: &mut PvmHooks<'_>, step_bounds: Bound<usize>) -> usize
    where
        M: state_backend::ManagerReadWrite,
    {
        self.machine_state
            .step_max_handle::<Infallible>(step_bounds, |machine_state, exc| {
                Ok(sbi::handle_call(
                    &mut self.status,
                    machine_state,
                    hooks,
                    exc,
                ))
            })
            .steps
    }

    /// Respond to a request for input with no input. Returns `false` in case the
    /// machine wasn't expecting any input, otherwise returns `true`.
    pub fn provide_no_input(&mut self) -> bool
    where
        M: state_backend::ManagerReadWrite,
    {
        sbi::provide_no_input(&mut self.status, &mut self.machine_state)
    }

    /// Provide input. Returns `false` if the machine state is not expecting
    /// input.
    pub fn provide_input(&mut self, level: u32, counter: u32, payload: &[u8]) -> bool
    where
        M: state_backend::ManagerReadWrite,
    {
        sbi::provide_input(
            &mut self.status,
            &mut self.machine_state,
            level,
            counter,
            payload,
        )
    }

    /// Provide metadata in response to a metadata request. Returns `false`
    /// if the machine is not expecting metadata.
    pub fn provide_metadata(&mut self, rollup_address: &[u8; 20], origination_level: u32) -> bool
    where
        M: state_backend::ManagerReadWrite,
    {
        sbi::provide_metadata(
            &mut self.status,
            &mut self.machine_state,
            rollup_address,
            origination_level,
        )
    }

    /// Get the current machine status.
    pub fn status(&self) -> PvmStatus
    where
        M: state_backend::ManagerRead,
    {
        self.status.read()
    }
}

impl<
        ML: main_memory::MainMemoryLayout,
        CL: machine_state::CacheLayouts,
        M: state_backend::ManagerClone,
    > Clone for Pvm<ML, CL, M>
{
    fn clone(&self) -> Self {
        Self {
            version: self.version.clone(),
            machine_state: self.machine_state.clone(),
            status: self.status.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        machine_state::{
            bus::main_memory::M1M,
            registers::{a0, a1, a2, a3, a6, a7},
            TestCacheLayouts,
        },
        state_backend::owned_backend::Owned,
    };
    use rand::{thread_rng, Fill};
    use std::mem;
    use tezos_smart_rollup_constants::riscv::{
        SBI_CONSOLE_PUTCHAR, SBI_FIRMWARE_TEZOS, SBI_TEZOS_INBOX_NEXT,
    };

    #[test]
    fn test_read_input() {
        type ML = M1M;
        type L = PvmLayout<ML, TestCacheLayouts>;

        // Setup PVM
        let space = Owned::allocate::<L>();
        let mut pvm = Pvm::<ML, TestCacheLayouts, _>::bind(space);
        pvm.reset();

        let level_addr = main_memory::FIRST_ADDRESS;
        let counter_addr = level_addr + 4;
        let buffer_addr = counter_addr + 4;

        const BUFFER_LEN: usize = 1024;

        // Configure machine for 'sbi_tezos_inbox_next'
        pvm.machine_state
            .core
            .hart
            .xregisters
            .write(a0, buffer_addr);
        pvm.machine_state
            .core
            .hart
            .xregisters
            .write(a1, BUFFER_LEN as u64);
        pvm.machine_state.core.hart.xregisters.write(a2, level_addr);
        pvm.machine_state
            .core
            .hart
            .xregisters
            .write(a3, counter_addr);
        pvm.machine_state
            .core
            .hart
            .xregisters
            .write(a7, SBI_FIRMWARE_TEZOS);
        pvm.machine_state
            .core
            .hart
            .xregisters
            .write(a6, SBI_TEZOS_INBOX_NEXT);

        // Should be in evaluating mode
        assert_eq!(pvm.status(), PvmStatus::Evaluating);

        // Handle the ECALL successfully
        let outcome =
            pvm.handle_exception(&mut Default::default(), EnvironException::EnvCallFromUMode);
        assert!(!outcome);

        // After the ECALL we should be waiting for input
        assert_eq!(pvm.status(), PvmStatus::WaitingForInput);

        // Respond to the request for input
        let level = rand::random();
        let counter = rand::random();
        let mut payload = [0u8; BUFFER_LEN + 10];
        payload.try_fill(&mut thread_rng()).unwrap();
        assert!(pvm.provide_input(level, counter, &payload));

        // The status should switch from WaitingForInput to Evaluating
        assert_eq!(pvm.status(), PvmStatus::Evaluating);

        // Returned data is as expected
        assert_eq!(
            pvm.machine_state.core.hart.xregisters.read(a0) as usize,
            BUFFER_LEN
        );
        assert_eq!(
            pvm.machine_state.core.main_memory.read(level_addr),
            Ok(level)
        );
        assert_eq!(
            pvm.machine_state.core.main_memory.read(counter_addr),
            Ok(counter)
        );

        // Payload in memory should be as expected
        for (offset, &byte) in payload[..BUFFER_LEN].iter().enumerate() {
            let addr = buffer_addr + offset as u64;
            let byte_written: u8 = pvm.machine_state.core.main_memory.read(addr).unwrap();
            assert_eq!(
                byte, byte_written,
                "Byte at {addr:x} (offset {offset}) is not the same"
            );
        }

        // Data after the buffer should be untouched
        assert!((BUFFER_LEN..4096)
            .map(|offset| {
                let addr = buffer_addr + offset as u64;
                pvm.machine_state.core.main_memory.read(addr).unwrap()
            })
            .all(|b: u8| b == 0));
    }

    #[test]
    fn test_write_debug() {
        type ML = M1M;
        type L = PvmLayout<ML, TestCacheLayouts>;

        let mut buffer = Vec::new();
        let mut hooks = PvmHooks::new(|c| buffer.push(c));

        // Setup PVM
        let space = Owned::allocate::<L>();
        let mut pvm = Pvm::<ML, TestCacheLayouts, _>::bind(space);
        pvm.reset();

        // Prepare subsequent ECALLs to use the SBI_CONSOLE_PUTCHAR extension
        pvm.machine_state
            .core
            .hart
            .xregisters
            .write(a7, SBI_CONSOLE_PUTCHAR);

        // Write characters
        let mut written = Vec::new();
        for _ in 0..10 {
            let char: u8 = rand::random();
            pvm.machine_state
                .core
                .hart
                .xregisters
                .write(a0, char as u64);
            written.push(char);

            let outcome = pvm.handle_exception(&mut hooks, EnvironException::EnvCallFromUMode);
            assert!(outcome, "Unexpected outcome");
        }

        // Drop `hooks` to regain access to the mutable references it kept
        mem::drop(hooks);

        // Compare what characters have been passed to the hook verrsus what we
        // intended to write
        assert_eq!(written, buffer);
    }
}
