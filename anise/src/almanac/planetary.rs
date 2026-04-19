/*
 * ANISE Toolkit
 * Copyright (C) 2021-onward Christopher Rabotin <christopher.rabotin@gmail.com> et al. (cf. AUTHORS.md)
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 *
 * Documentation: https://nyxspace.com/
 */

#[cfg(feature = "std")]
use hifitime::Epoch;

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
#[cfg(not(feature = "std"))]
use alloc::string::String;
use super::Almanac;
use crate::{
    prelude::{Frame, FrameUid},
    structure::{
        dataset::DataSetError, lookuptable::LutError, planetocentric::PlanetaryData,
        PlanetaryDataSet,
    },
    NaifId,
};
use log::warn;
use snafu::prelude::*;
#[cfg(feature = "std")]
#[cfg(feature = "std")]
#[cfg(feature = "std")]
use tabled::{settings::Style, Table, Tabled};

#[derive(Debug, Snafu, PartialEq)]
#[snafu(visibility(pub(crate)))]
#[non_exhaustive]
pub enum PlanetaryDataError {
    #[snafu(display("when {action}, {source}"))]
    PlanetaryDataSet {
        action: &'static str,
        source: DataSetError,
    },
}

impl Almanac {
    /// Given the frame UID (or something that can be transformed into it), attempt to retrieve the full frame information, if that frame is loaded
    #[deprecated(since = "0.7.0", note = "use frame_info instead")]
    pub fn frame_from_uid<U: Into<FrameUid>>(&self, uid: U) -> Result<Frame, PlanetaryDataError> {
        self.frame_info(uid)
    }

    /// Given the frame UID (or something that can be transformed into it), attempt to retrieve the full frame information, if that frame is loaded
    pub fn frame_info<U: Into<FrameUid>>(&self, uid: U) -> Result<Frame, PlanetaryDataError> {
        let uid = uid.into();
        for data in self.planetary_data.values().rev() {
            if let Ok(datum) = data.get_by_id(uid.ephemeris_id) {
                return Ok(datum.to_frame(uid));
            }
        }

        Err(PlanetaryDataError::PlanetaryDataSet {
            action: "fetching frame by its UID via ephemeris_id",
            source: DataSetError::DataSetLut {
                action: "fetching by ID",
                source: LutError::UnknownId {
                    id: uid.ephemeris_id,
                },
            },
        })
    }

    /// Returns the plantary from its ID, searching through all loaded planetary datasets in reverse order.
    pub fn get_planetary_data_from_id(
        &self,
        id: NaifId,
    ) -> Result<PlanetaryData, PlanetaryDataError> {
        for data in self.planetary_data.values().rev() {
            if let Ok(datum) = data.get_by_id(id) {
                return Ok(datum);
            }
        }

        Err(PlanetaryDataError::PlanetaryDataSet {
            action: "fetching planetary data via its id",
            source: DataSetError::DataSetLut {
                action: "fetching by ID",
                source: LutError::UnknownId { id },
            },
        })
    }

    /// Returns the plantary from its ID, searching through all loaded planetary datasets in reverse order.
    pub fn set_planetary_data_from_id(
        &mut self,
        id: NaifId,
        planetary_data: PlanetaryData,
    ) -> Result<(), PlanetaryDataError> {
        for data in self.planetary_data.values_mut().rev() {
            if data.set_by_id(id, planetary_data).is_ok() {
                // This dataset contained the ID, and we've set it correctly.
                return Ok(());
            }
        }

        Err(PlanetaryDataError::PlanetaryDataSet {
            action: "setting planetary data via its id",
            source: DataSetError::DataSetLut {
                action: "setting by ID",
                source: LutError::UnknownId { id },
            },
        })
    }

    /// Loads the provided planetary data.
    pub fn with_planetary_data(self, planetary_data: PlanetaryDataSet) -> Self {
        self.with_planetary_data_as(planetary_data, None)
    }

    /// Loads the provided planetary data.
    pub fn with_planetary_data_as(
        mut self,
        planetary_data: PlanetaryDataSet,
        alias: Option<String>,
    ) -> Self {
        // For lifetime reasons, we format the message using a ref first.
        // This message is only displayed if there was something with that name before.
        let alias = alias.unwrap_or(alloc::string::String::from("unknown"));
        let msg = format!("unloading planetary data `{alias}`");
        if self.planetary_data.insert(alias, planetary_data).is_some() {
            warn!("{msg}");
        }
        self
    }
}

#[derive(Default)]
#[cfg_attr(feature = "std", derive(Tabled))]
struct PlanetaryRow {
    #[cfg_attr(feature = "std", tabled(rename = "Name"))]
    name: String,
    #[cfg_attr(feature = "std", tabled(rename = "ID"))]
    id: String,
    #[cfg_attr(feature = "std", tabled(rename = "Gravity param (km^3/s^2)"))]
    gm: String,
    #[cfg_attr(feature = "std", tabled(rename = "Major axis (km)"))]
    major_axis: String,
    #[cfg_attr(feature = "std", tabled(rename = "Minor axis (km)"))]
    minor_axis: String,
    #[cfg_attr(feature = "std", tabled(rename = "Polar axis (km)"))]
    polar_axis: String,
    #[cfg_attr(feature = "std", tabled(rename = "Pole right asc."))]
    pole_ra: String,
    #[cfg_attr(feature = "std", tabled(rename = "Pole declination"))]
    pole_decl: String,
    #[cfg_attr(feature = "std", tabled(rename = "Prime meridian"))]
    pm: String,
}

impl PlanetaryDataSet {
    /// Returns a table describing this planetary data set
    pub fn describe(&self) -> String {
        let binding = self.lut.entries();
        let mut values = binding.values().collect::<Vec<_>>().to_vec();
        values.sort_by_key(|(opt_id, _)| match opt_id {
            Some(id) => *id,
            None => 0,
        });

        let mut rows = Vec::new();

        for (opt_id, opt_name) in values {
            let data = if let Some(id) = opt_id {
                self.get_by_id(*id).unwrap()
            } else {
                self.get_by_name(opt_name.as_ref().unwrap().as_str())
                    .unwrap()
            };

            let mut row = PlanetaryRow {
                name: match opt_name {
                    Some(name) => alloc::string::String::from(name.as_str()),
                    None => alloc::string::String::from("Unset"),
                },
                id: match opt_id {
                    Some(id) => format!("{id}"),
                    None => alloc::string::String::from("Unset"),
                },
                gm: format!("{}", data.mu_km3_s2),
                pole_ra: match data.pole_right_ascension {
                    None => alloc::string::String::from("Unset"),
                    Some(pole_ra) => format!("{pole_ra}"),
                },
                pole_decl: match data.pole_declination {
                    None => alloc::string::String::from("Unset"),
                    Some(pole_dec) => format!("{pole_dec}"),
                },
                pm: match data.prime_meridian {
                    None => alloc::string::String::from("Unset"),
                    Some(pm) => format!("{pm}"),
                },
                major_axis: alloc::string::String::from("Unset"),
                minor_axis: alloc::string::String::from("Unset"),
                polar_axis: alloc::string::String::from("Unset"),
            };

            match data.shape {
                None => {
                    // Three empty columns -- don't change the data
                }
                Some(shape) => {
                    row.major_axis = format!("{}", shape.semi_major_equatorial_radius_km);
                    row.minor_axis = format!("{}", shape.semi_minor_equatorial_radius_km);
                    row.polar_axis = format!("{}", shape.polar_radius_km);
                }
            }

            rows.push(row);
        }

        #[cfg(feature = "std")]
        {
            let mut tbl = Table::new(rows);
            tbl.with(Style::modern());
            format!("{tbl}")
        }
        #[cfg(not(feature = "std"))]
        alloc::string::String::from("Tabled output unavailable without std feature")
    }
}
