use std::{fmt::Display, time::Instant};

use crate::{
    controller::MoveStatus,
    physical::Physical,
    position::{PositionMM, PositionStepFloat},
};

/// Solve for 7-stage s-curve
///
/// stage_0: max_jerk until max_acceleration
/// stage_1: max_acceleration until almost max_velocity
/// stage_2: negative max_jerk until zero acceleration and max_velocity
/// stage_3: coast at max_velocity
/// stage_4: negative max_jerk until negative max_acceleration
/// stage_5: negative max_acceleration until nearly zero velocity
/// stage_6: max_jerk until zero acceleration and zero velocity
pub struct SCurveSolver {
    /// Max velocity
    // m_v: f64,
    /// Max acceleration
    m_a: f64,
    /// Max jerk
    m_j: f64,
    /// Time of max jerk stages (0, 2, 4, and 6)
    t_j0: f64,
    /// Time of max acceleration stages (1 and 5)
    t_v1: f64,
    /// The minimum distance in mm of a truncated coast (full) s-curve
    min_dist_truncated_coast: f64,
    /// The minimum distance in mm of a truncated max_accerleration (mid) s-curve
    min_dist_truncated_max_acceleration: f64,
}

impl Display for SCurveSolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "m_a: {}, m_j: {}, t_j0: {}, t_v1: {}, min_dist_full: {}, min_dist_mid: {}",
            self.m_a,
            self.m_j,
            self.t_j0,
            self.t_v1,
            self.min_dist_truncated_coast,
            self.min_dist_truncated_max_acceleration
        )
    }
}

impl SCurveSolver {
    /// m_a: Max acceleration
    /// m_j: Max jerk
    pub fn new(physical: &Physical, m_a: f64, m_j: f64) -> Self {
        let m_v = physical.get_max_velocity() as f64;
        let t_j0 = (m_a / m_j).min((m_v * 2.0 / m_j).sqrt());
        let t_v1 = (-m_j * t_j0.powi(2) + m_v) / m_a;
        let min_dist_truncated_coast = -m_a * t_j0.powi(2) / 2.0
            + m_j * t_j0.powi(3)
            + m_j * t_j0.powi(2) * t_v1 / 2.0
            + t_j0 * (m_a * t_v1 + m_j * t_j0.powi(2) / 2.0)
            + t_j0 * (m_a * t_v1 + m_j * t_j0.powi(2))
            + t_v1 * (m_a * t_v1 + m_j * t_j0.powi(2) / 2.0);
        let min_dist_truncated_max_acceleration =
            -m_a * t_j0.powi(2) / 2.0 + 5.0 * m_j * t_j0.powi(3) / 2.0;
        assert!(min_dist_truncated_coast >= min_dist_truncated_max_acceleration);
        SCurveSolver {
            // m_v,
            m_a,
            m_j,
            t_j0,
            t_v1,
            min_dist_truncated_coast,
            min_dist_truncated_max_acceleration,
        }
    }
    pub fn solve_curve(&self, start: PositionMM, end: PositionMM) -> SCurve {
        let dist = start.dist(&end);
        if dist >= self.min_dist_truncated_coast {
            self.solve_truncated_coast_curve(start, end)
        } else if dist > self.min_dist_truncated_max_acceleration {
            self.solve_truncated_max_acceleration_curve(start, end)
        } else {
            self.solve_truncated_max_jerk_curve(start, end)
        }
    }
    fn solve_truncated_coast_curve(&self, start: PositionMM, end: PositionMM) -> SCurve {
        let p = start.dist(&end);
        let t_c3 = (self.m_a * self.t_j0.powi(2) / 2.0
            - 2.0 * self.m_a * self.t_j0 * self.t_v1
            - self.m_a * self.t_v1.powi(2)
            - 5.0 * self.m_j * self.t_j0.powi(3) / 2.0
            - self.m_j * self.t_j0.powi(2) * self.t_v1
            + p)
            / (self.m_a * self.t_v1 + self.m_j * self.t_j0.powi(2));
        SCurve::new(start, end, self.t_j0, self.t_v1, t_c3, self)
    }
    fn solve_truncated_max_acceleration_curve(&self, start: PositionMM, end: PositionMM) -> SCurve {
        let p = start.dist(&end);
        let t_v1 = (-self.t_j0 * (2.0 * self.m_a + self.m_j * self.t_j0)
            + (6.0 * self.m_a.powi(2) * self.t_j0.powi(2)
                - 6.0 * self.m_a * self.m_j * self.t_j0.powi(3)
                + 4.0 * self.m_a * p
                + self.m_j.powi(2) * self.t_j0.powi(4))
            .sqrt())
            / (2.0 * self.m_a);
        assert!(t_v1 > 0.0, "Need to add in second solution");
        SCurve::new(start, end, self.t_j0, t_v1, 0.0, self)
    }
    fn solve_truncated_max_jerk_curve(&self, start: PositionMM, end: PositionMM) -> SCurve {
        let p = start.dist(&end);
        let t_j0 = 2.0_f64.powf(2.0 / 3.0) * (p / self.m_j).powf(1.0 / 3.0) / 2.0;
        SCurve::new(start, end, t_j0, 0.0, 0.0, self)
    }
}

pub struct SCurve {
    start: PositionMM,
    t_start: Instant,
    t: [f64; 7],
    a_j0: f64,
    v: [f64; 7],
    p: [f64; 7],
    dir: [f64; 2],
}

impl Display for SCurve {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let print_arr = |a: &[f64]| {
            a.iter()
                .map(|x| format!("{:.2}", x))
                .collect::<Vec<_>>()
                .join(", ")
        };
        write!(
            f,
            "start: {}, t:[{}], a_j0: {}, v: [{}], p: [{}], dir: [{}]",
            self.start,
            print_arr(&self.t),
            self.a_j0,
            print_arr(&self.v),
            print_arr(&self.p),
            print_arr(&self.dir),
        )
    }
}

impl Default for SCurve {
    fn default() -> Self {
        SCurve {
            start: PositionMM::default(),
            t_start: Instant::now(),
            t: [f64::default(); 7],
            a_j0: f64::default(),
            v: [f64::default(); 7],
            p: [f64::default(); 7],
            dir: [f64::default(); 2],
        }
    }
}

impl SCurve {
    pub fn new(
        start: PositionMM,
        end: PositionMM,
        t_j0: f64,
        t_v1: f64,
        t_c3: f64,
        solver: &SCurveSolver,
    ) -> Self {
        let mut t = [0.0; 7];
        t[0] = t_j0;
        t[1] = t[0] + t_v1;
        t[2] = t[1] + t_j0;
        t[3] = t[2] + t_c3;
        t[4] = t[3] + t_j0;
        t[5] = t[4] + t_v1;
        t[6] = t[5] + t_j0;
        let mut v = [0.0; 7];
        let mut p = [0.0; 7];
        let a_j0 = solver.m_j * t_j0;
        v[0] = solver.m_j * t_j0.powi(2) / 2.0;
        p[0] = solver.m_j * t_j0.powi(3) / 6.0;
        v[1] = v[0] + solver.m_a * t_v1;
        p[1] = p[0] + v[0] * t_v1 + solver.m_a * t_v1.powi(2) / 2.0;
        v[2] = v[1] + a_j0 * t_j0 - solver.m_j * t_j0.powi(2) / 2.0;
        p[2] = p[1] + v[1] * t_j0 + a_j0 * t_j0.powi(2) / 2.0 - solver.m_j * t_j0.powi(3) / 6.0;
        p[3] = p[2] + v[2] * t_c3;
        let a_j4 = -solver.m_j * t_j0;
        v[4] = v[2] - solver.m_j * t_j0.powi(2) / 2.0;
        p[4] = p[3] + v[2] * t_j0 - solver.m_j * t_j0.powi(3) / 6.0;
        v[5] = v[4] - solver.m_a * t_v1;
        p[5] = p[4] + v[4] * t_v1 - solver.m_a * t_v1.powi(2) / 2.0;
        p[6] = p[5] + v[5] * t_j0 + a_j4 * t_j0.powi(2) / 2.0 + solver.m_j * t_j0.powi(3) / 6.0;
        let dir = start.get_direction(&end);
        SCurve {
            start: start,
            t_start: Instant::now(),
            t,
            a_j0,
            v,
            p,
            dir,
        }
    }
    /// Return if we are in the process of moving or not
    pub fn get_move_status(&self) -> MoveStatus {
        let elasped = self.t_start.elapsed().as_secs_f64();
        if elasped >= self.t[self.t.len() - 1] {
            MoveStatus::Stopped
        } else {
            MoveStatus::Moving
        }
    }
    /// Return the desired step of the motors
    pub fn get_desired(&self, solver: &SCurveSolver) -> PositionStepFloat {
        let elasped = self.t_start.elapsed().as_secs_f64();
        let p = if elasped < self.t[0] {
            self.stage_0(elasped, solver)
        } else if elasped < self.t[1] {
            self.stage_1(elasped, solver)
        } else if elasped < self.t[2] {
            self.stage_2(elasped, solver)
        } else if elasped < self.t[3] {
            self.stage_3(elasped)
        } else if elasped < self.t[4] {
            self.stage_4(elasped, solver)
        } else if elasped < self.t[5] {
            self.stage_5(elasped, solver)
        } else {
            self.stage_6(elasped, solver)
        };
        let dist = self.dir.iter().map(|d| d * p);
        let mut desired = self.start.iter().zip(dist).map(|(&s, d)| s + d);
        PositionStepFloat::new([desired.next().unwrap(), desired.next().unwrap()])
    }

    fn stage_0(&self, elasped: f64, solver: &SCurveSolver) -> f64 {
        solver.m_j * elasped.powi(3) / 6.0
    }
    fn stage_1(&self, elasped: f64, solver: &SCurveSolver) -> f64 {
        let t = elasped - self.t[0];
        self.p[0] + self.v[0] * t + solver.m_a * t.powi(2) / 2.0
    }
    fn stage_2(&self, elasped: f64, solver: &SCurveSolver) -> f64 {
        let t = elasped - self.t[1];
        self.p[1] + self.v[1] * t + self.a_j0 * t.powi(2) / 2.0 - solver.m_j * t.powi(3) / 6.0
    }
    fn stage_3(&self, elasped: f64) -> f64 {
        let t = elasped - self.t[2];
        self.p[2] + self.v[2] * t
    }
    fn stage_4(&self, elasped: f64, solver: &SCurveSolver) -> f64 {
        let t = elasped - self.t[3];
        self.p[3] + self.v[2] * t - solver.m_j * t.powi(3) / 6.0
    }
    fn stage_5(&self, elasped: f64, solver: &SCurveSolver) -> f64 {
        let t = elasped - self.t[4];
        self.p[4] + self.v[4] * t - solver.m_a * t.powi(2) / 2.0
    }
    fn stage_6(&self, elasped: f64, solver: &SCurveSolver) -> f64 {
        let t = elasped - self.t[5];
        self.p[5] + self.v[5] * t - solver.m_a * t.powi(2) / 2.0 + solver.m_j * t.powi(3) / 6.0
    }
}
