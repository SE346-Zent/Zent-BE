use std::fmt;

/// The four canonical phases of a work-order closing workflow.
///
/// Every uploaded closing-form photo must carry one of these phases.
/// Completing a work order requires at least one photo per phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkOrderPhase {
    PreAssembly,
    Disassembled,
    PostAssembly,
    Signature,
}

impl WorkOrderPhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            WorkOrderPhase::PreAssembly => "pre-assembly",
            WorkOrderPhase::Disassembled => "disassembled",
            WorkOrderPhase::PostAssembly => "post-assembly",
            WorkOrderPhase::Signature => "signature",
        }
    }

    /// Parse a phase string (case-insensitive).
    /// Returns `None` for unrecognised input.
    pub fn from_str(s: &str) -> Option<WorkOrderPhase> {
        match s.trim().to_lowercase().as_str() {
            "pre-assembly" | "preassembly" => Some(WorkOrderPhase::PreAssembly),
            "disassembled" => Some(WorkOrderPhase::Disassembled),
            "post-assembly" | "postassembly" => Some(WorkOrderPhase::PostAssembly),
            "signature" => Some(WorkOrderPhase::Signature),
            _ => None,
        }
    }

    /// All valid phase variants.
    pub fn all() -> &'static [WorkOrderPhase] {
        &[
            WorkOrderPhase::PreAssembly,
            WorkOrderPhase::Disassembled,
            WorkOrderPhase::PostAssembly,
            WorkOrderPhase::Signature,
        ]
    }
}

impl fmt::Display for WorkOrderPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
