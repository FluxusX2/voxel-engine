pub struct Spiral {
    x: i32,
    y: i32,
    dx: i32,
    dy: i32,
    steps_in_leg: i32,
    leg_length: i32,
    leg_count: i32,
}

impl Spiral {
    pub fn new() -> Self {
        Spiral {
            x: 0,
            y: 0,
            dx: 0,
            dy: 1, // Start by going "Up" (0, 1)
            steps_in_leg: 0,
            leg_length: 1,
            leg_count: 0,
        }
    }
}

impl Iterator for Spiral {
    type Item = (i32, i32);

    fn next(&mut self) -> Option<Self::Item> {
        let current = (self.x, self.y);

        self.x += self.dx;
        self.y += self.dy;
        self.steps_in_leg += 1;

        if self.steps_in_leg == self.leg_length {
            let new_dx = self.dy;
            let new_dy = -self.dx;
            self.dx = new_dx;
            self.dy = new_dy;

            self.steps_in_leg = 0;
            self.leg_count += 1;

            if self.leg_count % 2 == 0 {
                self.leg_length += 1;
            }
        }

        Some(current)
    }
}