//! Tensor-product Gordon interpolation of intersecting polynomial guide curves.
use crate::*;
use operations::V3;
fn basis(t: f64, i: usize, n: usize) -> f64 {
    let mut c = 1.;
    for j in 1..=i {
        c *= ((n + 1 - j) as f64) / (j as f64);
    }
    c * t.powi(i as i32) * (1. - t).powi((n - i) as i32)
}
fn bezier(controls: &[Vec3], t: f64) -> Vec3 {
    controls.iter().enumerate().fold(Vec3::ZERO, |sum, (i, p)| {
        sum + *p * basis(t, i, controls.len() - 1)
    })
}
fn lagrange(stations: &[f64], at: f64) -> Vec<f64> {
    stations
        .iter()
        .enumerate()
        .map(|(i, s)| {
            stations
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, other)| (at - other) / (s - other))
                .product()
        })
        .collect()
}
// Convert samples on a uniform grid to Bernstein coefficients using pivoted elimination.
fn controls(mut samples: Vec<Vec3>) -> Result<Vec<Vec3>> {
    let degree = samples.len() - 1;
    let mut a = (0..=degree)
        .map(|i| {
            (0..=degree)
                .map(|j| basis(i as f64 / degree as f64, j, degree))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    for i in 0..=degree {
        let pivot = (i..=degree)
            .max_by(|&j, &k| a[j][i].abs().total_cmp(&a[k][i].abs()))
            .unwrap();
        a.swap(i, pivot);
        samples.swap(i, pivot);
        let scale = a[i][i];
        if scale.abs() < 1e-12 {
            return Err(CadError::new("INVALID_NETWORK", "singular interpolation"));
        }
        for x in &mut a[i] {
            *x /= scale;
        }
        samples[i] /= scale;
        let row = a[i].clone();
        let value = samples[i];
        for j in 0..=degree {
            if j == i {
                continue;
            }
            let factor = a[j][i];
            for (k, x) in a[j].iter_mut().enumerate() {
                *x -= factor * row[k];
            }
            samples[j] -= value * factor;
        }
    }
    Ok(samples)
}
fn family(
    curves: &[Vec<V3>],
    stations: &[f64],
    p: &BTreeMap<String, f64>,
) -> Result<Vec<Vec<Vec3>>> {
    if !(2..=8).contains(&curves.len())
        || stations.len() != curves.len()
        || stations.first() != Some(&0.)
        || stations.last() != Some(&1.)
        || stations
            .windows(2)
            .any(|w| !w[1].is_finite() || w[1] - w[0] < 1e-3)
    {
        return Err(CadError::new(
            "INVALID_NETWORK",
            "2..8 guides, strictly increasing stations from zero to one",
        ));
    }
    curves
        .iter()
        .map(|c| {
            if !(2..=8).contains(&c.len()) {
                return Err(CadError::new("INVALID_NETWORK", "Bezier degree 1..7"));
            }
            c.iter().map(|v| point(v, p)).collect()
        })
        .collect()
}
pub(crate) fn interpolate(
    u_curves: &[Vec<V3>],
    v_curves: &[Vec<V3>],
    u_stations: &[f64],
    v_stations: &[f64],
    p: &BTreeMap<String, f64>,
) -> Result<Vec<Vec<Vec3>>> {
    let us = family(u_curves, v_stations, p)?;
    let vs = family(v_curves, u_stations, p)?;
    let nu = us.iter().map(Vec::len).max().unwrap().max(vs.len());
    let nv = vs.iter().map(Vec::len).max().unwrap().max(us.len());
    let mut crossings = vec![vec![Vec3::ZERO; us.len()]; vs.len()];
    for (i, vc) in vs.iter().enumerate() {
        for (j, uc) in us.iter().enumerate() {
            let a = bezier(uc, u_stations[i]);
            let b = bezier(vc, v_stations[j]);
            if (a - b).length() > 1e-7 {
                return Err(CadError::new(
                    "NETWORK_MISMATCH",
                    format!("guides {i}/{j} do not intersect at declared parameters"),
                ));
            }
            crossings[i][j] = a;
        }
    }
    let mut along_v = Vec::with_capacity(nu);
    for iu in 0..nu {
        let u = iu as f64 / (nu - 1) as f64;
        let lu = lagrange(u_stations, u);
        let mut row = Vec::with_capacity(nv);
        for iv in 0..nv {
            let v = iv as f64 / (nv - 1) as f64;
            let lv = lagrange(v_stations, v);
            let mut q = Vec3::ZERO;
            for (j, c) in us.iter().enumerate() {
                q += bezier(c, u) * lv[j];
            }
            for (i, c) in vs.iter().enumerate() {
                q += bezier(c, v) * lu[i];
                for (j, l) in lv.iter().enumerate() {
                    q -= crossings[i][j] * (lu[i] * l);
                }
            }
            row.push(q);
        }
        along_v.push(controls(row)?);
    }
    let mut net = vec![vec![Vec3::ZERO; nv]; nu];
    for j in 0..nv {
        let column = controls(along_v.iter().map(|r| r[j]).collect())?;
        for (i, value) in column.into_iter().enumerate() {
            if !value.length().is_finite() || value.length() > MAX_COORD * 2. {
                return Err(CadError::new(
                    "SURFACE_BUDGET",
                    "network exceeds coordinate budget",
                ));
            }
            net[i][j] = value;
        }
    }
    Ok(net)
}
