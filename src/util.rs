use bevy::time::Timer;

pub(crate) struct TimerMs<const N: i32> {
    pub(crate) timer: Timer,
}

impl<const N: i32> Default for TimerMs<N> {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(N as f32 / 1000.0, bevy::time::TimerMode::Repeating),
        }
    }
}
