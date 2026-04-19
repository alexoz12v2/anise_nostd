#[cfg(not(feature = "std"))]
use alloc::string::String;
use snafu::prelude::*;

use crate::{
    errors::{DecodingError, IntegrityError},
    structure::lookuptable::LutError,
};
#[cfg(feature = "std")]
use std::io::Error as IOError;

#[cfg(not(feature = "std"))]
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct IOError;

#[cfg(not(feature = "std"))]
impl core::fmt::Display for IOError {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "IOError")
    }
}
#[cfg(not(feature = "std"))]
impl core::error::Error for IOError {}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
#[non_exhaustive]
pub enum DataSetError {
    #[snafu(display("when {action}, {source}"))]
    DataSetLut {
        action: &'static str,
        source: LutError,
    },
    #[snafu(display("when {action}, {source}"))]
    DataSetIntegrity {
        action: &'static str,
        source: IntegrityError,
    },
    #[snafu(display("when {action}, {source}"))]
    DataDecoding {
        action: &'static str,
        source: DecodingError,
    },
    #[snafu(display("input/output error while {action}, {source}"))]
    IO {
        action: &'static str,
        source: IOError,
    },
    #[snafu(display("data set conversion error: {action}"))]
    Conversion { action: String },
}

impl PartialEq for DataSetError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::DataSetLut {
                    action: l_action,
                    source: l_source,
                },
                Self::DataSetLut {
                    action: r_action,
                    source: r_source,
                },
            ) => l_action == r_action && l_source == r_source,
            (
                Self::DataSetIntegrity {
                    action: l_action,
                    source: l_source,
                },
                Self::DataSetIntegrity {
                    action: r_action,
                    source: r_source,
                },
            ) => l_action == r_action && l_source == r_source,
            (
                Self::DataDecoding {
                    action: l_action,
                    source: l_source,
                },
                Self::DataDecoding {
                    action: r_action,
                    source: r_source,
                },
            ) => l_action == r_action && l_source == r_source,
            (
                Self::IO {
                    action: l_action,
                    source: _l_source,
                },
                Self::IO {
                    action: r_action,
                    source: _r_source,
                },
            ) => l_action == r_action,
            _ => false,
        }
    }
}
