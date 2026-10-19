//! Animasi pegas teredam, supaya buka/tutup notch terasa "hidup" dan sedikit memantul.

pub struct Spring {
    pub value: f32,
    pub target: f32,
    velocity: f32,
    stiffness: f32,
    damping: f32,
    precision: f32,
}

impl Spring {
    /// `damping_ratio` < 1 memantul, = 1 berhenti mulus tanpa pantulan. `precision` adalah
    /// selisih yang sudah tidak terlihat (misal 0,05 DIP); di bawahnya animasi dianggap selesai.
    pub fn new(value: f32, stiffness: f32, damping_ratio: f32, precision: f32) -> Self {
        Self {
            value,
            target: value,
            velocity: 0.0,
            stiffness,
            damping: 2.0 * damping_ratio * stiffness.sqrt(),
            precision,
        }
    }

    /// Maju `dt` detik. Dipecah jadi langkah kecil supaya stabil walau frame telat.
    pub fn step(&mut self, dt: f32) {
        const SUBSTEP: f32 = 1.0 / 480.0;
        let mut left = dt.clamp(0.0, 0.05);
        while left > 0.0 {
            let h = left.min(SUBSTEP);
            let accel = -self.stiffness * (self.value - self.target) - self.damping * self.velocity;
            self.velocity += accel * h;
            self.value += self.velocity * h;
            left -= h;
        }
        if (self.value - self.target).abs() < self.precision && self.velocity.abs() < self.precision * 10.0 {
            self.snap(self.target);
        }
    }

    /// Langsung ke `value` tanpa animasi.
    pub fn snap(&mut self, value: f32) {
        self.value = value;
        self.target = value;
        self.velocity = 0.0;
    }

    pub fn is_moving(&self) -> bool {
        self.value != self.target || self.velocity != 0.0
    }
}
