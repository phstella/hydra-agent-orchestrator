pub mod parallel;
mod single;

pub use parallel::{ParallelHandle, ParallelResult, ParallelSupervisor, TaggedEvent};
pub use single::{
    AgentCommand, ProcessStatus, ProcessSupervisor, SupervisorConfig, SupervisorEvent,
    SupervisorHandle, SupervisorResult, TimeoutReason,
};
