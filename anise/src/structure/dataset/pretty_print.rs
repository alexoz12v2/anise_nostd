#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
#[cfg(not(feature = "std"))]
use alloc::string::String;
#[cfg(feature = "std")]
#[cfg(feature = "std")]
#[cfg(feature = "std")]
use tabled::{settings::Style, Table, Tabled};

use crate::structure::{EulerParameterDataSet, LocationDataSet};

use super::NaifId;

#[derive(Default)]
#[cfg_attr(feature = "std", derive(Tabled))]
struct EulerParamRow {
    #[cfg_attr(feature = "std", tabled(rename = "Name"))]
    name: String,
    #[cfg_attr(feature = "std", tabled(rename = "ID"))]
    id: String,
    #[cfg_attr(feature = "std", tabled(rename = "Quat w"))]
    qw: f64,
    #[cfg_attr(feature = "std", tabled(rename = "Quat x"))]
    qx: f64,
    #[cfg_attr(feature = "std", tabled(rename = "Quat y"))]
    qy: f64,
    #[cfg_attr(feature = "std", tabled(rename = "Quat z"))]
    qz: f64,
    #[cfg_attr(feature = "std", tabled(rename = "To ID"))]
    to: NaifId,
    #[cfg_attr(feature = "std", tabled(rename = "From ID"))]
    from: NaifId,
}

impl EulerParameterDataSet {
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

            let row = EulerParamRow {
                name: match opt_name {
                    Some(name) => alloc::string::String::from(name.as_str()),
                    None => alloc::string::String::from("Unset"),
                },
                id: match opt_id {
                    Some(id) => format!("{id}"),
                    None => alloc::string::String::from("Unset"),
                },
                qw: data.w,
                qx: data.x,
                qy: data.y,
                qz: data.z,
                to: data.to,
                from: data.from,
            };

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

#[derive(Default)]
#[cfg_attr(feature = "std", derive(Tabled))]
struct LocationRow {
    #[cfg_attr(feature = "std", tabled(rename = "Name"))]
    name: String,
    #[cfg_attr(feature = "std", tabled(rename = "ID"))]
    id: String,
    #[cfg_attr(feature = "std", tabled(rename = "Latitude (deg)"))]
    latitude_deg: f64,
    #[cfg_attr(feature = "std", tabled(rename = "Longitude (deg)"))]
    longitude_deg: f64,
    #[cfg_attr(feature = "std", tabled(rename = "Height (km)"))]
    height_km: f64,
    #[cfg_attr(feature = "std", tabled(rename = "Terrain Mask ?"))]
    has_terrain_mask: bool,
    #[cfg_attr(feature = "std", tabled(rename = "Terrain Mask Ignored"))]
    terrain_mask_ignored: bool,
}

impl LocationDataSet {
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

            let row = LocationRow {
                name: match opt_name {
                    Some(name) => alloc::string::String::from(name.as_str()),
                    None => alloc::string::String::from("Unset"),
                },
                id: match opt_id {
                    Some(id) => format!("{id}"),
                    None => alloc::string::String::from("Unset"),
                },
                latitude_deg: data.latitude_deg,
                longitude_deg: data.longitude_deg,
                height_km: data.height_km,
                has_terrain_mask: !data.terrain_mask.is_empty(),
                terrain_mask_ignored: data.terrain_mask_ignored,
            };

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
