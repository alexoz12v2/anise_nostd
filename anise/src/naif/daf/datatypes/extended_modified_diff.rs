/*
 * ANISE Toolkit — SPK Type 21 (Extended Modified Difference Array)
 * Ported from CSPICE spke21.c / spkr21.c (NablaZeroLabs/cspice).
 * No-std compatible.
 *
 * Key difference from Type 1 (modified_diff.rs):
 *   - MAXDIM is a runtime value stored as slice[len-2] in the segment data.
 *     Type 1 hardcodes 15 nodes; Type 21 supports up to MAXDIM=25.
 *   - Record (difference line) size is (4*MAXDIM + 11) doubles.
 *   - DT array is col-major [MAXDIM x 3].
 *   - W-accumulation uses while(ks >= 2) saving jx for the velocity step.
 */

use core::fmt;
use hifitime::Epoch;
use snafu::{ensure, ResultExt};

use crate::errors::{DecodingError, InaccessibleBytesSnafu, IntegrityError, TooFewDoublesSnafu};
use crate::math::interpolation::{InterpDecodingSnafu, InterpolationError};
use crate::naif::daf::NAIFSummaryRecord;
use crate::{
    math::Vector3,
    naif::daf::{NAIFDataRecord, NAIFDataSet},
};

/// Maximum MAXDIM supported (= CSPICE MAXTRM = 25).
const MAX_MAXDIM: usize = 25;

// ── Dataset (full segment) ───────────────────────────────────────────────────

#[derive(PartialEq)]
pub struct ExtendedModifiedDiffType21<'a> {
    pub maxdim: usize,
    pub num_records: usize,
    pub record_data: &'a [f64],
    pub epoch_data: &'a [f64],
    pub epoch_registry: &'a [f64],
}

impl fmt::Display for ExtendedModifiedDiffType21<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Extended MDA Type 21 (MAXDIM={}) from {:E} to {:E} with {} records",
            self.maxdim,
            Epoch::from_et_seconds(*self.epoch_data.first().unwrap_or(&0.0)),
            Epoch::from_et_seconds(*self.epoch_data.last().unwrap_or(&0.0)),
            self.num_records,
        )
    }
}

impl<'a> NAIFDataSet<'a> for ExtendedModifiedDiffType21<'a> {
    type StateKind = (Vector3, Vector3);
    type RecordKind = ExtendedModifiedDiffRecord<'a>;
    const DATASET_NAME: &'static str = "Extended Modified Differences Type 21";

    fn from_f64_slice(slice: &'a [f64]) -> Result<Self, DecodingError> {
        ensure!(
            slice.len() >= 4,
            TooFewDoublesSnafu {
                dataset: Self::DATASET_NAME,
                need: 4_usize,
                got: slice.len()
            }
        );
        // Tail layout confirmed from spkr21.c:
        //   slice[len-2] = maxdim  (MAXDIM stored at segment tail)
        //   slice[len-1] = num_records
        let num_records = slice[slice.len() - 1] as usize;
        let maxdim = slice[slice.len() - 2] as usize;

        ensure!(
            maxdim >= 1 && maxdim <= MAX_MAXDIM,
            InaccessibleBytesSnafu {
                start: 0_usize,
                end: maxdim,
                size: MAX_MAXDIM
            }
        );

        // dflsiz: doubles per difference line
        let dflsiz = 4 * maxdim + 11;
        let record_end = num_records * dflsiz;

        ensure!(
            record_end + num_records <= slice.len().saturating_sub(2),
            InaccessibleBytesSnafu {
                start: 0_usize,
                end: record_end + num_records + 2,
                size: slice.len(),
            }
        );

        let record_data = &slice[..record_end];
        let epoch_data = &slice[record_end..record_end + num_records];
        let epoch_registry = &slice[record_end + num_records..slice.len() - 2];

        Ok(Self {
            maxdim,
            num_records,
            record_data,
            epoch_data,
            epoch_registry,
        })
    }

    fn nth_record(&self, n: usize) -> Result<Self::RecordKind, DecodingError> {
        let dflsiz = 4 * self.maxdim + 11;
        let start = n * dflsiz;
        let end = start + dflsiz;
        Ok(ExtendedModifiedDiffRecord::from_slice_with_maxdim(
            self.record_data
                .get(start..end)
                .ok_or(DecodingError::InaccessibleBytes {
                    start,
                    end,
                    size: self.record_data.len(),
                })?,
            self.maxdim,
        ))
    }

    fn evaluate<S: NAIFSummaryRecord>(
        &self,
        epoch: Epoch,
        _: &S,
    ) -> Result<Self::StateKind, InterpolationError> {
        if self.epoch_data.is_empty() {
            return Err(InterpolationError::MissingInterpolationData { epoch });
        }
        let et = epoch.to_et_seconds();
        if et < self.epoch_data[0] - 1e-2
            || et > *self.epoch_data.last().unwrap() + 1e-2
        {
            return Err(InterpolationError::NoInterpolationData {
                req: epoch,
                start: Epoch::from_et_seconds(self.epoch_data[0]),
                end: Epoch::from_et_seconds(*self.epoch_data.last().unwrap()),
            });
        }
        // Pick first record whose epoch is strictly > et (same as Type 1)
        let idx = self.epoch_data.partition_point(|&e| e <= et);
        let record = self.nth_record(idx).context(InterpDecodingSnafu)?;
        Ok(record.to_pos_vel(epoch))
    }

    fn check_integrity(&self) -> Result<(), IntegrityError> {
        for &v in self
            .record_data
            .iter()
            .chain(self.epoch_data)
            .chain(self.epoch_registry)
        {
            if !v.is_finite() {
                return Err(IntegrityError::SubNormal {
                    dataset: Self::DATASET_NAME,
                    variable: "record or epoch data",
                });
            }
        }
        Ok(())
    }
}

// ── Record (one difference line) ─────────────────────────────────────────────

/// A single extended difference line record from an SPK Type 21 segment.
///
/// Layout within the dflsiz slice (0-indexed, no leading MAXDIM word):
/// ```text
/// [0]                     TL   — reference epoch (ET seconds)
/// [1 .. 1+maxdim]         G    — step-size vector
/// [1+maxdim]              REFPOS[0]   x (km)
/// [2+maxdim]              REFVEL[0]   vx (km/s)
/// [3+maxdim]              REFPOS[1]   y
/// [4+maxdim]              REFVEL[1]   vy
/// [5+maxdim]              REFPOS[2]   z
/// [6+maxdim]              REFVEL[2]   vz
/// [7+maxdim .. 7+4*maxdim]  DT[maxdim x 3] col-major (x-col, y-col, z-col)
/// [7+4*maxdim]            KQMAX1
/// [8+4*maxdim]            KQ[0]
/// [9+4*maxdim]            KQ[1]
/// [10+4*maxdim]           KQ[2]
/// ```
#[derive(Clone, Debug)]
pub struct ExtendedModifiedDiffRecord<'a> {
    pub ref_epoch: f64,
    pub g: &'a [f64],
    pub ref_x_km: f64,
    pub ref_vx_km_s: f64,
    pub ref_y_km: f64,
    pub ref_vy_km_s: f64,
    pub ref_z_km: f64,
    pub ref_vz_km_s: f64,
    pub dt: &'a [f64],
    pub kqmax1: usize,
    pub kq: [usize; 3],
    pub maxdim: usize,
}

impl<'a> ExtendedModifiedDiffRecord<'a> {
    fn from_slice_with_maxdim(slice: &'a [f64], maxdim: usize) -> Self {
        Self {
            ref_epoch: slice[0],
            g: &slice[1..1 + maxdim],
            ref_x_km: slice[1 + maxdim],
            ref_vx_km_s: slice[2 + maxdim],
            ref_y_km: slice[3 + maxdim],
            ref_vy_km_s: slice[4 + maxdim],
            ref_z_km: slice[5 + maxdim],
            ref_vz_km_s: slice[6 + maxdim],
            dt: &slice[7 + maxdim..7 + 4 * maxdim],
            kqmax1: slice[7 + 4 * maxdim] as usize,
            kq: [
                slice[8 + 4 * maxdim] as usize,
                slice[9 + 4 * maxdim] as usize,
                slice[10 + 4 * maxdim] as usize,
            ],
            maxdim,
        }
    }

    /// Evaluate position and velocity at `epoch`.
    ///
    /// Direct port of CSPICE spke21.c. Fortran 1-based indexing translated to 0-based.
    pub fn to_pos_vel(&self, epoch: Epoch) -> (Vector3, Vector3) {
        let et = epoch.to_et_seconds();
        let delta = et - self.ref_epoch;
        let kqmax1 = self.kqmax1;
        let maxdim = self.maxdim;

        // fc[1..=maxdim] (fc[0] unused — Fortran 1-indexed)
        let mut fc = [0.0f64; MAX_MAXDIM + 1];
        let mut wc = [0.0f64; MAX_MAXDIM];

        let mq2 = kqmax1 as isize - 2;
        let mut tp = delta;

        for j in 1..=(mq2.max(0) as usize) {
            let g_j = self.g[j - 1];
            fc[j] = tp / g_j;
            wc[j - 1] = delta / g_j;
            tp = delta + g_j;
        }

        let mut w = [0.0f64; MAX_MAXDIM + 2];
        for j in 1..=kqmax1 {
            w[j - 1] = 1.0 / j as f64;
        }

        // W(K) for position — while(ks >= 2), tracking jx
        let mut ks: isize = kqmax1 as isize - 1;
        let mut ks1: isize = ks - 1;
        let mut jx: usize = 0;
        while ks >= 2 {
            jx += 1;
            for j in 1..=jx {
                let i_ks = (j as isize + ks - 1) as usize;
                let i_ks1 = (j as isize + ks1 - 1) as usize;
                w[i_ks] = fc[j] * w[i_ks1] - wc[j - 1] * w[i_ks];
            }
            ks = ks1;
            ks1 -= 1;
        }
        // After loop: ks = 1, ks1 = 0

        let mut pos_km = Vector3::zeros();
        let mut vel_km_s = Vector3::zeros();

        // Position interpolation
        for i in 0..3usize {
            let kqq = self.kq[i];
            let mut sum = 0.0f64;
            for j in (1..=kqq).rev() {
                let dt_idx = j - 1 + i * maxdim;
                let w_idx = (j as isize + ks - 1) as usize;
                sum += self.dt[dt_idx] * w[w_idx];
            }
            let (refpos, refvel) = match i {
                0 => (self.ref_x_km, self.ref_vx_km_s),
                1 => (self.ref_y_km, self.ref_vy_km_s),
                2 => (self.ref_z_km, self.ref_vz_km_s),
                _ => unreachable!(),
            };
            pos_km[i] = refpos + delta * (refvel + delta * sum);
        }

        // W(K) update for velocity
        for j in 1..=jx {
            let i_ks = (j as isize + ks - 1) as usize;
            let i_ks1 = (j as isize + ks1 - 1) as usize;
            w[i_ks] = fc[j] * w[i_ks1] - wc[j - 1] * w[i_ks];
        }
        ks -= 1; // ks = 0

        // Velocity interpolation
        for i in 0..3usize {
            let kqq = self.kq[i];
            let mut sum = 0.0f64;
            for j in (1..=kqq).rev() {
                let dt_idx = j - 1 + i * maxdim;
                let w_idx = (j as isize + ks - 1) as usize;
                sum += self.dt[dt_idx] * w[w_idx];
            }
            let refvel = match i {
                0 => self.ref_vx_km_s,
                1 => self.ref_vy_km_s,
                2 => self.ref_vz_km_s,
                _ => unreachable!(),
            };
            vel_km_s[i] = refvel + delta * sum;
        }

        (pos_km, vel_km_s)
    }
}

impl fmt::Display for ExtendedModifiedDiffRecord<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Type21 record @ TL={} maxdim={} kqmax1={}",
            self.ref_epoch, self.maxdim, self.kqmax1
        )
    }
}

impl<'a> NAIFDataRecord<'a> for ExtendedModifiedDiffRecord<'a> {
    fn from_slice_f64(slice: &'a [f64]) -> Self {
        // Infer maxdim from slice length: dflsiz = 4*maxdim + 11
        let maxdim = slice.len().saturating_sub(11) / 4;
        Self::from_slice_with_maxdim(slice, maxdim)
    }
}
