//! This module contains traits for the VM interface.
//! All VMs should implement these traits to be used in our system.
//! The trait is generic over the storage type, allowing it to be used with any storage implementation.
//! Additionally, this trait is generic over HistoryMode, allowing it to be used with or without history.
//!
//! `TracerDispatcher` is an associated type used to dispatch tracers in VMs.
//! It manages tracers across different VM versions.
//! Even though we use the same interface for all VM versions,
//! we can now specify only the necessary trait bounds for each VM version.
//!
//! Generally speaking, in most cases, the tracer dispatcher is a wrapper around `Vec<Box<dyn VmTracer>>`,
//! where `VmTracer` is a trait implemented for a specific VM version.
//!
//! Example usage:
//! ```
//! use std::{
//!     cell::RefCell,
//!     rc::Rc,
//!     sync::Arc
//! };
//! use once_cell::sync::OnceCell;
//! use multivm::{
//!     interface::{L1BatchEnv, SystemEnv, VmInterface},
//!     tracers::CallTracer ,
//!     vm_latest::ToTracerPointer
//! };
//! use zksync_state::{InMemoryStorage, StorageView};
//! use zksync_types::Transaction;
//!
//! // Prepare the environment for the VM.
//! let l1_batch_env = L1BatchEnv::new();
//! let system_env = SystemEnv::default();
//! // Create storage
//! let storage = Rc::new(RefCell::new(StorageView::new(InMemoryStorage::default())));
//! // Instantiate VM with the desired version.
//! let mut vm = multivm::vm_latest::Vm::new(l1_batch_env, system_env, storage);
//! // Push a transaction to the VM.
//! let tx = Transaction::default();
//! vm.push_transaction(tx);
//! // Instantiate a tracer.
//! let result = Arc::new(OnceCell::new());
//! let call_tracer = CallTracer::new(result.clone()).into_tracer_pointer();
//! // Inspect the transaction with a tracer. You can use either one tracer or a vector of tracers.
//! let result = vm.inspect(call_tracer.into(), multivm::interface::VmExecutionMode::OneTx);
//!
//! // To obtain the result of the entire batch, you can use the following code:
//! let result = vm.execute(multivm::interface::VmExecutionMode::Batch);
//! ```

use crate::interface::types::errors::BytecodeCompressionError;
use crate::interface::types::inputs::{L1BatchEnv, L2BlockEnv, SystemEnv, VmExecutionMode};
use crate::interface::types::outputs::{
    BootloaderMemory, CurrentExecutionState, VmExecutionResultAndLogs,
};

use crate::interface::{FinishedL1Batch, VmMemoryMetrics};
use crate::vm_latest::HistoryEnabled;
use crate::HistoryMode;
use zksync_state::StoragePtr;
use zksync_types::Transaction;
use zksync_utils::bytecode::CompressedBytecodeInfo;

/// Public interface for VM
pub trait VmInterface<S, H: HistoryMode> {
    type TracerDispatcher: Default;

    /// Initialize VM.
    fn new(batch_env: L1BatchEnv, system_env: SystemEnv, storage: StoragePtr<S>) -> Self;

    /// Push transaction to bootloader memory.
    fn push_transaction(&mut self, tx: Transaction);

    /// Execute next VM step (either next transaction or bootloader or the whole batch).
    fn execute(&mut self, execution_mode: VmExecutionMode) -> VmExecutionResultAndLogs {
        self.inspect(Self::TracerDispatcher::default(), execution_mode)
    }

    /// Execute next VM step (either next transaction or bootloader or the whole batch)
    /// with custom tracers.
    fn inspect(
        &mut self,
        dispatcher: Self::TracerDispatcher,
        execution_mode: VmExecutionMode,
    ) -> VmExecutionResultAndLogs;

    /// Get bootloader memory.
    fn get_bootloader_memory(&self) -> BootloaderMemory;

    /// Get last transaction's compressed bytecodes.
    fn get_last_tx_compressed_bytecodes(&self) -> Vec<CompressedBytecodeInfo>;

    /// Start a new L2 block.
    fn start_new_l2_block(&mut self, l2_block_env: L2BlockEnv);

    /// Get the current state of the virtual machine.
    fn get_current_execution_state(&self) -> CurrentExecutionState;

    /// Execute transaction with optional bytecode compression.
    fn execute_transaction_with_bytecode_compression(
        &mut self,
        tx: Transaction,
        with_compression: bool,
    ) -> Result<VmExecutionResultAndLogs, BytecodeCompressionError> {
        self.inspect_transaction_with_bytecode_compression(
            Self::TracerDispatcher::default(),
            tx,
            with_compression,
        )
    }

    /// Execute transaction with optional bytecode compression using custom tracers.
    fn inspect_transaction_with_bytecode_compression(
        &mut self,
        tracer: Self::TracerDispatcher,
        tx: Transaction,
        with_compression: bool,
    ) -> Result<VmExecutionResultAndLogs, BytecodeCompressionError>;

    fn record_vm_memory_metrics(&self) -> VmMemoryMetrics;
    fn finish_batch(&mut self) -> FinishedL1Batch;
}

/// Methods of vm, which required some history manipullations
pub trait VmInterfaceHistoryEnabled<S>: VmInterface<S, HistoryEnabled> {
    /// Create snapshot of current vm state and push it into the memory
    fn make_snapshot(&mut self);

    /// Roll back VM state to the latest snapshot and destroy the snapshot.
    fn rollback_to_the_latest_snapshot(&mut self);

    /// Pop the latest snapshot from memory and destroy it.
    fn pop_snapshot_no_rollback(&mut self);
}