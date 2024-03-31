use std::time::Instant;

use crate::{
    controller::MoveStatus,
    physical::Physical,
    position::{PositionStep, PositionUM},
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
    m_v: f64,
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
            m_v,
            m_a,
            m_j,
            t_j0,
            t_v1,
            min_dist_truncated_coast,
            min_dist_truncated_max_acceleration,
        }
    }
    pub fn solve_curve(&self, start: PositionUM, end: PositionUM) -> SCurve {
        let dist = start.dist(&end);
        if dist >= self.min_dist_truncated_coast {
            self.solve_truncated_coast_curve(start, end)
        } else if dist > self.min_dist_truncated_max_acceleration {
            self.solve_truncated_max_acceleration_curve(start, end)
        } else {
            self.solve_truncated_max_jerk_curve(start, end)
        }
    }
    fn solve_truncated_coast_curve(&self, start: PositionUM, end: PositionUM) -> SCurve {
        todo!();
    }
    fn solve_truncated_max_acceleration_curve(&self, start: PositionUM, end: PositionUM) -> SCurve {
        todo!();
    }
    fn solve_truncated_max_jerk_curve(&self, start: PositionUM, end: PositionUM) -> SCurve {
        todo!();
    }
}

pub struct SCurve {
    start: PositionUM,
    end: PositionUM,
    t_start: Instant,
    t_j0: f64,
    t_v1: f64,
    t_c3: f64,
    t: [f64; 7],
}

impl Default for SCurve {
    fn default() -> Self {
        SCurve {
            start: PositionUM::default(),
            end: PositionUM::default(),
            t_start: Instant::now(),
            t_j0: f64::default(),
            t_v1: f64::default(),
            t_c3: f64::default(),
            t: [f64::default(); 7],
        }
    }
}

impl SCurve {
    pub fn new(
        start: PositionUM,
        end: PositionUM,
        t_j0: f64,
        t_v1: f64,
        t_c3: f64,
        physical: &Physical,
    ) -> Self {
        let mut t = [0.0; 7];
        t[0] = t_j0;
        t[1] = t[0] + t_v1;
        t[2] = t[1] + t_j0;
        t[3] = t[2] + t_c3;
        t[4] = t[3] + t_j0;
        t[5] = t[4] + t_v1;
        t[6] = t[5] + t_j0;
        SCurve {
            start,
            end,
            t_start: Instant::now(),
            t_j0,
            t_v1,
            t_c3,
            t,
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
    pub fn get_desired(&self, now: &Instant, physical: &Physical) -> PositionStep {
        todo!();
    }
}
