use core::fmt;

use rusty_matter_sdf::SdfError;

/// ADF operation failure.
#[derive(Clone, Debug, PartialEq)]
pub enum AdfError {
    /// Source SDF grid operation failed.
    Sdf(SdfError),
    /// ADF schema was unexpected.
    UnexpectedSchema {
        /// Expected schema ID.
        expected: &'static str,
        /// Actual schema ID.
        actual: String,
    },
    /// Field ID was empty.
    EmptyFieldId,
    /// Source grid ID was empty.
    EmptySourceGridId,
    /// Field origin was non-finite.
    NonFiniteOrigin,
    /// Cell origin was non-finite.
    NonFiniteCellOrigin {
        /// Rejected cell index.
        index: usize,
    },
    /// Field or cell extent was invalid.
    InvalidExtent(f32),
    /// Maximum subdivision depth was invalid.
    InvalidMaxDepth(u32),
    /// Maximum leaf-cell budget was invalid.
    InvalidCellBudget,
    /// Maximum finest-grid index-cell budget was invalid.
    InvalidIndexGridBudget,
    /// Builder cell budget was exceeded.
    CellBudgetExceeded {
        /// Requested or minimum required leaf-cell count.
        requested: usize,
        /// Maximum allowed leaf-cell count.
        max: usize,
    },
    /// ADF finest-grid index budget was exceeded.
    IndexGridBudgetExceeded {
        /// Requested finest-grid lookup cell count.
        requested: usize,
        /// Maximum allowed finest-grid lookup cell count.
        max: usize,
    },
    /// ADF finest-grid index dimensions overflowed native addressable size.
    IndexGridOverflow,
    /// ADF leaf cell cannot be represented by the finest-grid index.
    IndexCellOutOfBounds {
        /// Rejected cell index.
        index: usize,
    },
    /// ADF finest-grid index does not cover every finest cell.
    IncompleteIndexGrid {
        /// Number of unassigned finest cells.
        missing: usize,
    },
    /// Error tolerance was invalid.
    InvalidErrorTolerance(f32),
    /// Field had no cells.
    EmptyCells,
    /// Cell level exceeded field maximum depth.
    CellLevelExceeded {
        /// Rejected cell index.
        index: usize,
        /// Cell level.
        level: u32,
        /// Field maximum depth.
        max_depth: u32,
    },
    /// Cell distance was non-finite.
    NonFiniteDistance {
        /// Rejected cell index.
        index: usize,
    },
    /// Cell distance bounds were invalid.
    InvalidDistanceRange {
        /// Rejected cell index.
        index: usize,
    },
}

impl fmt::Display for AdfError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sdf(error) => write!(formatter, "{error}"),
            Self::UnexpectedSchema { expected, actual } => {
                write!(formatter, "expected schema {expected}, found {actual}")
            }
            Self::EmptyFieldId => formatter.write_str("ADF field id must not be empty"),
            Self::EmptySourceGridId => formatter.write_str("ADF source grid id must not be empty"),
            Self::NonFiniteOrigin => formatter.write_str("ADF origin must be finite"),
            Self::NonFiniteCellOrigin { index } => {
                write!(formatter, "ADF cell {index} origin must be finite")
            }
            Self::InvalidExtent(value) => {
                write!(formatter, "ADF extent must be finite and positive: {value}")
            }
            Self::InvalidMaxDepth(value) => {
                write!(formatter, "ADF max_depth is too large: {value}")
            }
            Self::InvalidCellBudget => formatter.write_str("ADF max_cells must be non-zero"),
            Self::InvalidIndexGridBudget => {
                formatter.write_str("ADF index max_grid_cells must be non-zero")
            }
            Self::CellBudgetExceeded { requested, max } => {
                write!(formatter, "requested {requested} ADF cells, max is {max}")
            }
            Self::IndexGridBudgetExceeded { requested, max } => write!(
                formatter,
                "requested {requested} ADF index cells, max is {max}"
            ),
            Self::IndexGridOverflow => formatter.write_str("ADF index grid dimensions overflowed"),
            Self::IndexCellOutOfBounds { index } => {
                write!(formatter, "ADF cell {index} is outside the index grid")
            }
            Self::IncompleteIndexGrid { missing } => {
                write!(
                    formatter,
                    "ADF index grid has {missing} unassigned finest cells"
                )
            }
            Self::InvalidErrorTolerance(value) => {
                write!(
                    formatter,
                    "ADF error_tolerance must be finite and non-negative: {value}"
                )
            }
            Self::EmptyCells => formatter.write_str("ADF field must contain at least one cell"),
            Self::CellLevelExceeded {
                index,
                level,
                max_depth,
            } => write!(
                formatter,
                "ADF cell {index} level {level} exceeds max_depth {max_depth}"
            ),
            Self::NonFiniteDistance { index } => {
                write!(formatter, "ADF cell {index} distance is non-finite")
            }
            Self::InvalidDistanceRange { index } => {
                write!(formatter, "ADF cell {index} distance range is invalid")
            }
        }
    }
}

impl std::error::Error for AdfError {}

impl From<SdfError> for AdfError {
    fn from(value: SdfError) -> Self {
        Self::Sdf(value)
    }
}
