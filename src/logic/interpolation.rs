use std::collections::VecDeque;

use super::*;

#[derive(Debug, Clone)]
pub struct Interpolation {
    current: Vec2<f32>,
    targets: VecDeque<VecDeque<Vec2<f32>>>,
    targets_distance: f32,
}

impl Interpolation {
    pub fn new(pos: Vec2<f32>) -> Self {
        Self {
            current: pos,
            targets: {
                let mut targets = VecDeque::new();
                targets.push_front(VecDeque::new());
                targets
            },
            targets_distance: 0.0,
        }
    }

    pub fn current(&self) -> Vec2<f32> {
        self.current
    }

    pub fn queue(&mut self, pos: Vec2<f32>) {
        let targets = self.targets.back_mut().unwrap();
        if let Some(&last) = targets.back() {
            self.targets_distance += (pos - last).len();
        }
        targets.push_back(pos);
    }

    pub fn queue_jump(&mut self, pos: Vec2<f32>) {
        let mut jump = VecDeque::new();
        jump.push_back(pos);
        self.targets.push_back(jump);
    }

    pub fn update(&mut self, delta_time: f32) {
        let targets = &self.targets[0];
        if targets.is_empty() && self.targets.len() > 1 {
            // Jump
            self.targets.pop_front();
            let next = self.targets.get_mut(0).unwrap();
            self.current = next.pop_front().unwrap();
            if let Some(&new_next) = next.front() {
                self.targets_distance -= (new_next - self.current).len();
            }
            return;
        }
        if let Some(&next) = targets.front() {
            // Interpolate
            let delta = next - self.current;
            let distance = delta.len();
            let max_speed = (INTERPOLATION_MIN_SPEED)
                .max((distance + self.targets_distance) / INTERPOLATION_MAX_TIME)
                * delta_time;
            if distance <= max_speed {
                // Reached target
                let current = self.targets.front_mut().unwrap();
                current.pop_front();
                if let Some(&new_next) = current.front() {
                    self.targets_distance -= (new_next - next).len();
                }
                self.current += delta;
            } else {
                self.current += delta / distance * max_speed;
            }
        }
    }
}
