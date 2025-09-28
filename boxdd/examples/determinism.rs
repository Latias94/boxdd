use boxdd::prelude::*;
use std::time::Instant;

// Determinism headless sample: run the same seeded scenario twice and compare aggregates.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let seed: u64 = 0xDEADBEEFCAFEBABE;
    let steps = 240usize;
    let (sum1, dur1) = run_once(seed, steps)?;
    let (sum2, dur2) = run_once(seed, steps)?;
    let equal = (sum1.0.to_bits(), sum1.1.to_bits()) == (sum2.0.to_bits(), sum2.1.to_bits());
    println!(
        "determinism: equal={} pos_sum={:.6} vel_sum={:.6} time_ms=({:.3},{:.3})",
        equal,
        sum1.0,
        sum1.1,
        dur1.as_secs_f64() * 1000.0,
        dur2.as_secs_f64() * 1000.0
    );
    Ok(())
}

fn run_once(
    seed: u64,
    steps: usize,
) -> Result<((f32, f32), std::time::Duration), Box<dyn std::error::Error>> {
    let mut rng = Lcg64::new(seed);
    let mut world = World::new(
        WorldDef::builder()
            .gravity([0.0_f32, -10.0])
            .worker_count(1)
            .enable_continuous(true)
            .build(),
    )?;
    // Ground
    let ground = world.create_body_id(BodyBuilder::new().build());
    let _ = world.create_segment_shape_for(
        ground,
        &ShapeDef::builder().build(),
        &shapes::segment([-50.0_f32, 0.0], [50.0, 0.0]),
    );

    // Spawn a deterministic field of dynamic boxes and circles
    let sdef = ShapeDef::builder().density(1.0).build();
    let box_poly = shapes::box_polygon(0.25, 0.25);
    let circ = shapes::circle([0.0_f32, 0.0], 0.25);
    let mut bodies = Vec::with_capacity(200);
    for _ in 0..200 {
        let x = (rng.next_f32() - 0.5) * 20.0;
        let y = 1.0 + rng.next_f32() * 10.0;
        let t = rng.next_u32() % 2;
        let id = world.create_body_id(
            BodyBuilder::new()
                .body_type(BodyType::Dynamic)
                .position([x, y])
                .build(),
        );
        if t == 0 {
            let _ = world.create_polygon_shape_for(id, &sdef, &box_poly);
        } else {
            let _ = world.create_circle_shape_for(id, &sdef, &circ);
        }
        bodies.push(id);
    }

    let start = Instant::now();
    for _ in 0..steps {
        world.step(1.0 / 60.0, 8);
    }
    let dur = start.elapsed();

    // Aggregate sums
    let mut pos_sum = 0.0f32;
    let mut vel_sum = 0.0f32;
    for &b in &bodies {
        let p = world.body_position(b);
        let v = unsafe { boxdd_sys::ffi::b2Body_GetLinearVelocity(b) };
        pos_sum += p.x + p.y;
        vel_sum += v.x + v.y;
    }
    Ok(((pos_sum, vel_sum), dur))
}

struct Lcg64 {
    state: u64,
}
impl Lcg64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }
    fn next_u32(&mut self) -> u32 {
        ((self.next_u64() >> 32) & 0xffff_ffff) as u32
    }
    fn next_u64(&mut self) -> u64 {
        // PCG-XSH-RR like constants (LCG core), simplified
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.state
    }
    fn next_f32(&mut self) -> f32 {
        let bits = 0x3f80_0000 | (self.next_u32() >> 9); // [1.0,2.0)
        f32::from_bits(bits) - 1.0
    }
}
