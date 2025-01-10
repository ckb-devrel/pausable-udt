use core::str::Utf8Error;

use ckb_ssri_sdk::public_module_traits::udt::{UDTError, UDTPausableError};
use ckb_ssri_sdk::SSRIError;
use ckb_std::error::SysError;
use serde_molecule;

/// Error
#[repr(i8)]
#[derive(Debug)]
pub enum Error {
    // * CKB Error
    IndexOutOfBound = 1,
    ItemMissing = 2,
    LengthNotEnough,
    Encoding,
    SpawnExceededMaxContentLength,
    SpawnWrongMemoryLimit,
    SpawnExceededMaxPeakMemory,

    // * Rust Error
    Utf8Error,

    // * SSRI Error
    SSRIMethodsNotFound,
    SSRIMethodsArgsInvalid,
    SSRIMethodsNotImplemented,
    SSRIMethodRequireHigherLevel,
    InvalidVmVersion,

    // * Molecule Error
    MoleculeVerificationError,

    // * Serde Molecule Error
    SerdeMoleculeErrorWithMessage,
    /// Contains a general error message as a string.
    /// Occurs when the data length is incorrect while parsing a number or molecule header.
    MismatchedLength,
    /// Occurs when the data length is insufficient while parsing a number or molecule header.
    SerdeMoleculeLengthNotEnough,
    /// Indicates that the method or type is not implemented. Not all types in Rust can be serialized.
    Unimplemented,
    /// Occurs when assembling a molecule fixvec, and the size of each element is inconsistent.
    AssembleFixvec,
    /// Occurs when the header or size is incorrect while parsing a molecule fixvec.
    InvalidFixvec,
    /// Occurs when the field count is mismatched while parsing a molecule table.
    MismatchedTableFieldCount,
    /// Occurs when an overflow happens while parsing a molecule header.
    Overflow,
    /// Indicates an error encountered while parsing a molecule array.
    InvalidArray,
    /// Indicates that non-fixed size fields are not allowed in a molecule struct, e.g., `Option`, `Vec`, `DynVec`, `enum`.
    InvalidStructField,
    /// Indicates that a map should have exactly two fields: a key and a value.
    InvalidMap,
    /// Indicates that the table header is invalid or malformed.
    InvalidTable,
    /// Indicates that the table length is invalid or malformed.
    InvalidTableLength,
    /// Indicates that the table header is invalid or malformed.
    InvalidTableHeader,
    /// Indicates that the field count in serialization is mismatched.
    InvalidTableCount,
    /// Indicates that non-fixed size fields are not allowed in a molecule struct, e.g., `Option`, `Vec`, `DynVec`, `enum`.
    MixTableAndStruct,
    InvalidChar,

    // * UDT Error
    InsufficientBalance,
    NoTransferPermission,
    NoMintPermission,
    NoBurnPermission,

    // * UDT Pausable Error
    NoPausePermission,
    NoUnpausePermission,
    AbortedFromPause,
    IncompletePauseList,
    CyclicPauseList,
    InvalidPauseData,
}

#[allow(non_snake_case, unused)]
impl From<SysError> for Error {
    fn from(err: SysError) -> Self {
        use SysError::*;
        match err {
            IndexOutOfBound => Self::IndexOutOfBound,
            ItemMissing => Self::ItemMissing,
            LengthNotEnough(_) => Self::LengthNotEnough,
            Encoding => Self::Encoding,
            SpawnExceededMaxContentLength => Self::SpawnExceededMaxContentLength,
            SpawnWrongMemoryLimit => Self::SpawnWrongMemoryLimit,
            SpawnExceededMaxPeakMemory => Self::SpawnExceededMaxPeakMemory,
            Unknown(err_code) => panic!("unexpected sys error {}", err_code),
        }
    }
}

impl From<Utf8Error> for Error {
    fn from(_err: Utf8Error) -> Self {
        Self::Utf8Error
    }
}

impl From<serde_molecule::Error> for Error {
    fn from(err: serde_molecule::Error) -> Self {
        use serde_molecule::Error::*;
        match err {
            Message(_string) => Self::SerdeMoleculeErrorWithMessage,
            MismatchedLength => Self::MismatchedLength,
            LengthNotEnough => Self::SerdeMoleculeLengthNotEnough,
            Unimplemented => Self::Unimplemented,
            AssembleFixvec => Self::AssembleFixvec,
            InvalidFixvec => Self::InvalidFixvec,
            MismatchedTableFieldCount => Self::MismatchedTableFieldCount,
            Overflow => Self::Overflow,
            InvalidArray => Self::InvalidArray,
            InvalidStructField => Self::InvalidStructField,
            InvalidMap => Self::InvalidMap,
            InvalidTable => Self::InvalidTable,
            InvalidTableLength => Self::InvalidTableLength,
            InvalidTableHeader => Self::InvalidTableHeader,
            InvalidTableCount => Self::InvalidTableCount,
            MixTableAndStruct => Self::MixTableAndStruct,
            InvalidChar => Self::InvalidChar,
        }
    }
}

impl From<SSRIError> for Error {
    fn from(err: SSRIError) -> Self {
        match err {
            SSRIError::SSRIMethodsNotFound => Self::SSRIMethodsArgsInvalid,
            SSRIError::SSRIMethodsArgsInvalid => Self::SSRIMethodsNotImplemented,
            SSRIError::SSRIMethodsNotImplemented => Self::SSRIMethodsNotImplemented,
            SSRIError::SSRIMethodRequireHigherLevel => Self::SSRIMethodRequireHigherLevel,
            SSRIError::InvalidVmVersion => Self::InvalidVmVersion,
        }
    }
}

impl From<UDTError> for Error {
    fn from(err: UDTError) -> Self {
        match err {
            UDTError::InsufficientBalance => Self::InsufficientBalance,
            UDTError::NoMintPermission => Self::NoMintPermission,
            UDTError::NoBurnPermission => Self::NoBurnPermission,
        }
    }
}

impl From<UDTPausableError> for Error {
    fn from(err: UDTPausableError) -> Self {
        match err {
            UDTPausableError::NoPausePermission => Self::NoPausePermission,
            UDTPausableError::NoUnpausePermission => Self::NoUnpausePermission,
            UDTPausableError::AbortedFromPause => Self::AbortedFromPause,
            UDTPausableError::IncompletePauseList => Self::IncompletePauseList,
            UDTPausableError::CyclicPauseList => Self::CyclicPauseList,
        }
    }
}
