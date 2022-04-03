use std::collections::VecDeque;

use super::*;

#[derive(Debug, Clone)]
pub struct Interpolation {
    current: Vec2<f32>,
    targets: VecDeque<Vec2<f32>>,
    targets_distance: f32,
}

impl Interpolation {
    pub fn new(pos: Vec2<f32>) -> Self {
        Self {
            current: pos,
            targets: VecDeque::new(),
            targets_distance: 0.0,
        }
    }

    pub fn current(&self) -> Vec2<f32> {
        self.current
    }

    pub fn queue(&mut self, pos: Vec2<f32>) {
        if let Some(&last) = self.targets.back() {
            self.targets_distance += (pos - last).len();
        }
        self.targets.push_back(pos);
    }

    pub fn update(&mut self, delta_time: f32) {
        if let Some(&next) = self.targets.front() {
            let delta = next - self.current;
            let distance = delta.len();
            let max_speed = (INTERPOLATION_MIN_SPEED)
                .max((distance + self.targets_distance) / INTERPOLATION_MAX_TIME)
                * delta_time;
            if distance < max_speed {
                self.targets.pop_front();
                if let Some(&new_next) = self.targets.front() {
                    self.targets_distance -= (new_next - next).len();
                }
                self.current += delta;
            } else {
                self.current += delta / distance * max_speed;
            }
        }
    }
}
