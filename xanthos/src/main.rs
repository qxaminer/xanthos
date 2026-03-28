// TEACH: This is not just code. This is a Rust classroom
// disguised as a crab dreaming of arrays.
//
// Run:
//   cd xanthos
//   cargo run --release

use nannou::prelude::*;
use nannou_audio as audio;

use bytemuck::{Pod, Zeroable};
use std::sync::{Arc, Mutex};

// TEACH: Perlin implements `NoiseFn`, which provides the `.get([..])` method.
use nannou::noise::NoiseFn;

// ============================================================================
// STORY TEXT (THE CODE THAT SCROLLS)
// ============================================================================

// TEACH: `&'static str` is a string slice that lives for the entire program.
const CODE: &str = r#"
// Xanthos dreamed this before wgpu existed
@group(0) @binding(0) var<uniform> swirl: Swirl;

fn boid_to_child(fish: vec3<f32>, fear: f32) -> Crab {
    // the gonads know
    // the gonads always knew
    return Crab {
        pos: fish.xy,
        eyes: vec2<f32>(NO_LID, NO_LID),
        dream: fear * swirl.time,
    };
}

fn main() {
    let xanthos = Crab::new(Lidless, Lidless);

    loop {
        xanthos.dream(Fish::swirl());
        xanthos.wake();
        xanthos.climb();

        if let Some(berry) = surface.catch() {
            // this is the moment
            // before dissolution
            // before the deep reclaims
            xanthos.taste(berry);
        }
    }
}

// no rabbits
// no men
// no words
// just core
"#;

// ============================================================================
// AUDIO: BYTEMUCK AQUATIC SYNTH
// ============================================================================

// TEACH: `#[repr(C)]` guarantees field order/layout.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct StereoSample {
    left: f32,
    right: f32,
}

#[derive(Clone, Copy)]
struct Oscillator {
    phase: f32,      // 0..1, wraps
    frequency: f32,  // Hz
    amplitude: f32,  // 0..1
}

impl Oscillator {
    // TEACH: `&mut self` is an exclusive mutable borrow (compile-time race prevention).
    fn next_sample(&mut self, sample_rate: f32) -> f32 {
        let s = (self.phase * TAU).sin() * self.amplitude;
        // TEACH: `.fract()` keeps phase in 0..1 without `%`.
        self.phase = (self.phase + self.frequency / sample_rate).fract();
        s
    }
}

struct AquaticSynth {
    drone: Oscillator,
    shimmer: Oscillator,
    shimmer_mod: Oscillator,

    chime: Oscillator,
    chime_envelope: f32,

    grains: [Oscillator; 8],
    grain_envelope: f32,

    depth: f32,
    sample_rate: f32,
}

impl AquaticSynth {
    fn new(sample_rate: f32) -> Self {
        Self {
            drone: Oscillator {
                phase: 0.0,
                frequency: 40.0,
                amplitude: 0.30,
            },
            shimmer: Oscillator {
                phase: 0.0,
                frequency: 1200.0,
                amplitude: 0.0,
            },
            shimmer_mod: Oscillator {
                phase: 0.0,
                frequency: 0.30,
                amplitude: 400.0,
            },
            chime: Oscillator {
                phase: 0.0,
                frequency: 2400.0,
                amplitude: 0.50,
            },
            chime_envelope: 0.0,
            grains: [Oscillator {
                phase: 0.0,
                frequency: 300.0,
                amplitude: 0.10,
            }; 8],
            grain_envelope: 0.0,
            depth: 0.5,
            sample_rate,
        }
    }

    fn set_sample_rate_if_needed(&mut self, sr: f32) {
        if (self.sample_rate - sr).abs() > f32::EPSILON {
            self.sample_rate = sr;
        }
    }

    fn set_depth(&mut self, depth: f32) {
        self.depth = depth.clamp(0.0, 1.0);
        self.drone.frequency = 40.0 + (1.0 - self.depth) * 20.0;
        self.shimmer.amplitude = (1.0 - self.depth) * 0.15;
    }

    fn trigger_chime(&mut self) {
        self.chime_envelope = 1.0;
        self.chime.phase = 0.0;
    }

    fn trigger_transformation(&mut self) {
        self.grain_envelope = 1.0;
        for (i, g) in self.grains.iter_mut().enumerate() {
            g.frequency = 200.0 + i as f32 * 50.0;
            g.phase = i as f32 / 8.0;
        }
    }

    fn next_sample(&mut self) -> StereoSample {
        let sr = self.sample_rate;

        let drone = self.drone.next_sample(sr) * (0.5 + self.depth * 0.5);

        let modv = self.shimmer_mod.next_sample(sr);
        self.shimmer.frequency = 1200.0 + modv;
        let shimmer = self.shimmer.next_sample(sr);

        let chime = if self.chime_envelope > 0.001 {
            let s = self.chime.next_sample(sr) * self.chime_envelope;
            self.chime_envelope *= 0.9995;
            s
        } else {
            0.0
        };

        let grains = if self.grain_envelope > 0.001 {
            let sum: f32 = self
                .grains
                .iter_mut()
                .fold(0.0, |acc, g| acc + g.next_sample(sr));
            self.grain_envelope *= 0.9998;
            sum * self.grain_envelope / 8.0
        } else {
            0.0
        };

        let mix = drone + shimmer + chime + grains;
        let clipped = mix.tanh();

        StereoSample {
            left: clipped,
            right: (clipped + drone * 0.10).tanh(),
        }
    }

    fn fill_buffer(&mut self, out: &mut [StereoSample]) {
        for s in out.iter_mut() {
            *s = self.next_sample();
        }
    }
}

// TEACH: Audio callback (real-time thread). No allocations in here.
fn audio_render(synth: &mut Arc<Mutex<AquaticSynth>>, buffer: &mut audio::Buffer<f32>) {
    let channels = buffer.channels();
    let sr = buffer.sample_rate() as f32;

    let mut synth = match synth.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    synth.set_sample_rate_if_needed(sr);

    if channels == 2 {
        // TEACH: Buffer derefs to `&mut [f32]` (interleaved samples).
        let interleaved: &mut [f32] = &mut buffer[..];
        // TEACH: Stereo interleaving means the length is `frames * 2`.
        // We *assume* nannou_audio gives us whole frames; debug-assert documents that assumption.
        debug_assert!(interleaved.len() % 2 == 0);
        // TEACH: Safe, zero-copy cast: `[f32]` ↔ `[StereoSample]` (2 channels).
        let stereo: &mut [StereoSample] = bytemuck::cast_slice_mut(interleaved);
        synth.fill_buffer(stereo);
    } else {
        for s in buffer.iter_mut() {
            *s = 0.0;
        }
    }
}

// ============================================================================
// VISUALS: HELLO-UNIVERSE (TEACHING EDITION)
// ============================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Dream,
    Wake,
    Climb,
    Catch,
    Sink,
}

const TRAIL_LEN: usize = 14;

#[derive(Clone, Copy)]
struct Fish {
    pos: Vec2,
    vel: Vec2,
    morph: f32, // 0..1 fish->baby crab
    trail: [Vec2; TRAIL_LEN],
    trail_head: usize,
    glow: f32,
}

#[derive(Clone, Copy)]
struct BabyCrab {
    pos: Vec2,
    wobble_seed: f32,
}

struct Berry {
    pos: Vec2,
    vel: Vec2,
    size: f32,
    dissolving: bool,
}

struct Particle {
    pos: Vec2,
    vel: Vec2,
    life: f32,
    hue: f32,
    size: f32,
}

struct Bubble {
    pos: Vec2,
    vy: f32,
    life: f32,
    r: f32,
}

struct EyeStalk {
    base_local: Vec2,
    len: f32,
    tip_world: Vec2,
}

struct Xanthos {
    pos: Vec2,
    vel: Vec2,
    awake: bool,
    depth: f32,
    glow: f32,
    eyes: [EyeStalk; 2],
    just_caught_berry: bool,
}

struct Camera {
    target: Vec2,
    shake: Vec2,
    zoom: f32,
}

struct Model {
    phase: Phase,
    phase_t: f32,
    time: f32,
    depth: f32,

    // TEACH: `Vec<Fish>` OWNS its `Fish` values.
    // When `Model` is dropped, the Vec is dropped, and then every Fish inside is dropped.
    // Python contrast: a list typically holds references; GC cleans up later.
    fish: Vec<Fish>,
    baby_crabs: Vec<BabyCrab>,
    xanthos: Xanthos,

    // TEACH: `Option<T>` forces us to handle the “maybe missing” case.
    // This is Rust’s answer to `null` — the compiler *makes* you check.
    berry: Option<Berry>,

    particles: Vec<Particle>,
    bubbles: Vec<Bubble>,

    code_lines: Vec<&'static str>,
    code_scroll: f32,
    alpha_phase: f32,

    cam: Camera,
    perlin: nannou::noise::Perlin,

    synth: Arc<Mutex<AquaticSynth>>,
    _stream: audio::Stream<Arc<Mutex<AquaticSynth>>>,

    was_transforming: bool,
}

fn main() {
    nannou::app(model).update(update).run();
}

fn model(app: &App) -> Model {
    app.new_window()
        .size(1280, 800)
        .title("Xanthos Universe (Teaching Edition)")
        .view(view)
        .build()
        .unwrap();

    let win = app.window_rect();

    // AUDIO
    let synth = Arc::new(Mutex::new(AquaticSynth::new(44_100.0)));
    let stream = audio::Host::new()
        .new_output_stream(Arc::clone(&synth))
        .render(audio_render)
        .channels(2)
        .sample_rate(44_100)
        .frames_per_buffer(256)
        .build()
        // TEACH: `.expect(...)` will panic if building the stream fails.
        // For teaching + early prototypes, a clear panic message is okay.
        // In production, you'd return a `Result` and show a user-friendly error.
        .expect("failed to build audio stream");
    stream
        .play()
        // TEACH: same idea — if the OS refuses playback, we want a loud failure.
        .expect("failed to play audio stream");

    // VISUALS
    let mut fish = Vec::new();
    for i in 0..60 {
        let a = i as f32 * 0.22;
        let r = 90.0 + i as f32 * 2.2;
        let p = vec2(a.cos() * r, a.sin() * r - 60.0);
        fish.push(Fish {
            pos: p,
            vel: vec2(0.0, 0.0),
            morph: 0.0,
            trail: [p; TRAIL_LEN],
            trail_head: 0,
            glow: random_f32(),
        });
    }

    let xanthos = Xanthos {
        pos: vec2(0.0, win.bottom() + 140.0),
        vel: vec2(0.0, 0.0),
        awake: false,
        depth: 1.0,
        glow: 0.0,
        eyes: [
            EyeStalk {
                base_local: vec2(-14.0, 14.0),
                len: 28.0,
                tip_world: vec2(-14.0, 42.0),
            },
            EyeStalk {
                base_local: vec2(14.0, 14.0),
                len: 28.0,
                tip_world: vec2(14.0, 42.0),
            },
        ],
        just_caught_berry: false,
    };

    Model {
        phase: Phase::Dream,
        phase_t: 0.0,
        time: 0.0,
        depth: 1.0,

        fish,
        baby_crabs: Vec::new(),
        xanthos,
        berry: None,

        particles: Vec::new(),
        bubbles: Vec::new(),

        code_lines: CODE.lines().collect(),
        code_scroll: 0.0,
        alpha_phase: 0.0,

        cam: Camera {
            target: vec2(0.0, 0.0),
            shake: vec2(0.0, 0.0),
            zoom: 1.0,
        },
        perlin: nannou::noise::Perlin::new(),

        synth,
        _stream: stream,

        was_transforming: false,
    }
}

fn update(app: &App, model: &mut Model, update: Update) {
    let dt = update.since_last.as_secs_f32().min(1.0 / 30.0);
    model.time += dt;
    model.phase_t += dt;
    model.alpha_phase += dt * 1.2;
    model.xanthos.just_caught_berry = false;

    // Phase machine.
    match model.phase {
        Phase::Dream => {
            model.xanthos.awake = false;
            model.depth = 1.0;
            if model.phase_t > 4.0 {
                model.phase = Phase::Wake;
                model.phase_t = 0.0;
            }
        }
        Phase::Wake => {
            model.xanthos.awake = true;
            model.depth = 1.0 - ease_out_cubic((model.phase_t / 1.5).min(1.0)) * 0.35;
            if model.phase_t > 1.6 {
                model.phase = Phase::Climb;
                model.phase_t = 0.0;
            }
        }
        Phase::Climb => {
            model.xanthos.awake = true;
            model.depth = 0.65 - ease_in_out_cubic((model.phase_t / 4.0).min(1.0)) * 0.55;

            if model.berry.is_none() && random_f32() < dt * 0.25 {
                model.berry = Some(Berry {
                    pos: vec2(random_range(-app.window_rect().w() * 0.35, app.window_rect().w() * 0.35), app.window_rect().h() * 0.45),
                    vel: vec2(0.0, -28.0),
                    size: 14.0,
                    dissolving: false,
                });
            }

            if model.berry.is_some() && model.phase_t > 1.0 {
                model.phase = Phase::Catch;
                model.phase_t = 0.0;
            }
        }
        Phase::Catch => {
            model.xanthos.awake = true;
            model.depth = 0.12;
            if model.phase_t > 2.0 {
                model.phase = Phase::Sink;
                model.phase_t = 0.0;
            }
        }
        Phase::Sink => {
            model.xanthos.awake = false;
            model.depth = ease_in_cubic((model.phase_t / 4.0).min(1.0));
            if model.phase_t > 4.0 {
                model.phase = Phase::Dream;
                model.phase_t = 0.0;
            }
        }
    }

    model.xanthos.depth = model.depth;
    model.xanthos.glow = if model.xanthos.awake {
        (model.time * 2.3).sin().abs()
    } else {
        0.0
    };

    // Berry update + chase + catch.
    if let Some(berry) = model.berry.as_mut() {
        berry.pos += berry.vel * dt;
        berry.size *= 1.0 - dt * (0.08 + model.depth * 0.20);

        if model.xanthos.awake {
            let dir = (berry.pos - model.xanthos.pos).normalize_or_zero();
            model.xanthos.vel = lerp_vec2(model.xanthos.vel, dir * 120.0, 1.0 - (0.0001_f32).powf(dt));
        }

        if model.xanthos.awake && model.xanthos.pos.distance(berry.pos) < 42.0 {
            berry.dissolving = true;
        }

        if berry.dissolving || berry.size < 2.0 || berry.pos.y < -app.window_rect().h() * 0.55 {
            // Particles
            for _ in 0..16 {
                let a = random_f32() * TAU;
                let sp = random_range(40.0, 160.0);
                model.particles.push(Particle {
                    pos: berry.pos,
                    vel: vec2(a.cos(), a.sin()) * sp,
                    life: 1.0,
                    hue: random_range(0.55, 0.75),
                    size: random_range(1.5, 4.0),
                });
            }
            model.xanthos.just_caught_berry = berry.dissolving;
            model.cam.shake += vec2(random_range(-9.0, 9.0), random_range(-7.0, 7.0));
            model.berry = None;
        }
    }

    model.xanthos.pos += model.xanthos.vel * dt;
    model.xanthos.vel *= 0.92;

    // IK eyes.
    let look_target = model
        .berry
        .as_ref()
        .map(|b| b.pos)
        .unwrap_or_else(|| app.mouse.position());
    for eye in model.xanthos.eyes.iter_mut() {
        let base = model.xanthos.pos + eye.base_local;
        eye.tip_world = base + (look_target - base).normalize_or_zero() * eye.len;
    }

    // Boids.
    update_boids(&mut model.fish, model.xanthos.pos, model.xanthos.awake, dt, app.window_rect());
    for (i, f) in model.fish.iter_mut().enumerate() {
        f.trail[f.trail_head] = f.pos;
        f.trail_head = (f.trail_head + 1) % TRAIL_LEN;
        f.glow = 0.4 + 0.6 * (model.time * 2.1 + i as f32 * 0.3).sin().abs();
    }

    // Transform phase + sound trigger.
    let transforming = (model.time * 0.12).sin() > 0.55;
    if transforming && !model.was_transforming {
        model.was_transforming = true;
        if let Ok(mut s) = model.synth.lock() {
            s.trigger_transformation();
        }
    } else if !transforming {
        model.was_transforming = false;
    }

    for (i, f) in model.fish.iter_mut().enumerate() {
        let target = if transforming { 1.0 } else { 0.0 };
        f.morph = lerp(f.morph, target, 1.0 - (0.001_f32).powf(dt));
        if f.morph > 0.65 && (i % 10 == 0) && random_f32() < dt * 0.4 {
            model.baby_crabs.push(BabyCrab {
                pos: f.pos,
                wobble_seed: random_f32() * 10.0,
            });
        }
    }
    if model.baby_crabs.len() > 120 {
        model.baby_crabs.drain(0..(model.baby_crabs.len() - 120));
    }

    // Particles.
    for p in model.particles.iter_mut() {
        p.pos += p.vel * dt;
        p.vel *= 1.0 - dt * 1.2;
        p.life -= dt * (0.8 + model.depth);
    }
    model.particles.retain(|p| p.life > 0.0);

    // Bubbles.
    if random_f32() < dt * (0.4 + model.depth * 1.6) {
        model.bubbles.push(Bubble {
            pos: vec2(random_range(-app.window_rect().w() * 0.55, app.window_rect().w() * 0.55), -app.window_rect().h() * 0.55),
            vy: random_range(40.0, 120.0) * (0.6 + model.depth * 0.9),
            life: 1.0,
            r: random_range(2.0, 10.0),
        });
    }
    for b in model.bubbles.iter_mut() {
        b.pos.y += b.vy * dt;
        b.pos.x += (model.time * 1.8 + b.pos.y * 0.02).sin() * dt * 12.0;
        b.life -= dt * 0.35;
    }
    model.bubbles.retain(|b| b.life > 0.0 && b.pos.y < app.window_rect().h() * 0.55);

    // Code scroll + camera.
    model.code_scroll += dt * (32.0 + (1.0 - model.depth) * 18.0);
    model.cam.target = model.xanthos.pos;
    model.cam.zoom = 1.0 + (1.0 - model.depth) * 0.25;
    model.cam.shake *= 1.0 - dt * 8.0;

    // Sync audio depth + chime.
    if let Ok(mut s) = model.synth.lock() {
        s.set_depth(model.depth);
        if model.xanthos.just_caught_berry {
            s.trigger_chime();
        }
    }
}

fn view(app: &App, model: &Model, frame: Frame) {
    let draw = app.draw();
    let win = app.window_rect();
    draw.rect().wh(win.wh()).color(ocean_color(&model.perlin, model.time, model.depth));

    let cam_offset = model.cam.target + model.cam.shake;
    let zoom = model.cam.zoom;

    draw_scrolling_code(&draw, model, win, vec2(-win.w() * 0.35, 0.0) - cam_offset * 0.12, zoom);

    let world = draw.scale(zoom).translate(vec3(-cam_offset.x, -cam_offset.y, 0.0));

    for b in &model.bubbles {
        world
            .ellipse()
            .xy(b.pos)
            .radius(b.r)
            .color(srgba(0.7, 0.9, 1.0, b.life * 0.22));
    }

    for f in &model.fish {
        draw_fish_with_trail(&world, f, model.depth);
    }

    for c in &model.baby_crabs {
        draw_baby_crab(&world, c, model.time);
    }

    for p in &model.particles {
        let a = p.life.clamp(0.0, 1.0);
        world
            .ellipse()
            .xy(p.pos)
            .radius(p.size)
            .color(hsva(p.hue, 0.55, 0.95, a * 0.7));
    }

    if let Some(b) = model.berry.as_ref() {
        world
            .ellipse()
            .xy(b.pos)
            .w_h(b.size * 1.1, b.size)
            .color(srgba(0.25, 0.15, 0.65, 0.95));
    }

    draw_xanthos(&world, &model.xanthos, model.time);

    draw.text("Xanthos Universe (Teaching Edition)")
        .x_y(0.0, win.top() - 28.0)
        .font_size(20)
        .color(srgba(0.95, 0.9, 0.8, 0.8));

    draw.to_frame(app, &frame).unwrap();
}

fn draw_scrolling_code(draw: &Draw, model: &Model, win: Rect, origin: Vec2, zoom: f32) {
    let line_h = 18.0;
    let total_h = model.code_lines.len() as f32 * line_h;
    let scroll = model.code_scroll % total_h;
    let base_a = (model.alpha_phase.sin() * 0.5 + 0.5) * 0.14 + 0.06;
    let aberr = (1.0 - model.depth) * 2.0;

    for (i, &line) in model.code_lines.iter().enumerate() {
        let mut y = win.top() - 60.0 - i as f32 * line_h + scroll;
        if y > win.top() + 60.0 {
            y -= total_h;
        }
        if y < win.bottom() - 60.0 {
            y += total_h;
        }

        let local = base_a * (1.0 + (y * 0.01 + model.alpha_phase * 2.0).sin() * 0.5);
        let p = origin + vec2(0.0, y);

        let font_size = (14.0 / zoom).max(10.0) as u32;

        draw.text(line)
            .x_y(p.x + aberr, p.y)
            .font_size(font_size)
            .left_justify()
            .color(srgba(0.9, 0.25, 0.35, local * 0.45));
        draw.text(line)
            .x_y(p.x - aberr, p.y)
            .font_size(font_size)
            .left_justify()
            .color(srgba(0.25, 0.9, 0.55, local));
        draw.text(line)
            .x_y(p.x, p.y + aberr)
            .font_size(font_size)
            .left_justify()
            .color(srgba(0.2, 0.6, 1.0, local * 0.35));
    }
}

fn draw_fish_with_trail(draw: &Draw, f: &Fish, depth: f32) {
    for t in 0..TRAIL_LEN {
        let idx = (f.trail_head + t) % TRAIL_LEN;
        let p = f.trail[idx];
        let a = t as f32 / TRAIL_LEN as f32;
        let col = srgba(0.2, 0.9, 0.95, a * 0.25 * f.glow * (1.0 - depth * 0.6));
        draw.ellipse().xy(p).radius(2.0 + a * 1.2).color(col);
    }

    let fish_alpha = (1.0 - f.morph).clamp(0.0, 1.0);
    if fish_alpha > 0.01 {
        let ang = f.vel.angle();
        draw.ellipse()
            .xy(f.pos)
            .w_h(14.0, 7.0)
            .rotate(ang)
            .color(srgba(0.75, 0.8, 0.95, fish_alpha * 0.8));
        let tail = vec2(-10.0, 0.0).rotate(ang);
        draw.tri()
            .points(
                f.pos + tail,
                f.pos + tail + vec2(-6.0, 4.0).rotate(ang),
                f.pos + tail + vec2(-6.0, -4.0).rotate(ang),
            )
            .color(srgba(0.65, 0.7, 0.9, fish_alpha * 0.6));
    }
}

fn draw_baby_crab(draw: &Draw, c: &BabyCrab, time: f32) {
    let wobble = (time * 4.7 + c.wobble_seed).sin() * 0.2;
    draw.ellipse()
        .xy(c.pos)
        .w_h(10.0, 8.0)
        .rotate(wobble)
        .color(srgba(0.95, 0.35, 0.25, 0.55));
}

fn draw_xanthos(draw: &Draw, x: &Xanthos, time: f32) {
    if x.glow > 0.01 {
        draw.ellipse()
            .xy(x.pos)
            .w_h(120.0, 90.0)
            .color(srgba(1.0, 0.55, 0.25, x.glow * 0.20));
    }

    draw.ellipse()
        .xy(x.pos)
        .w_h(60.0, 42.0)
        .color(srgba(0.85, 0.25, 0.15, 0.92));

    // Eyes (IK stalks).
    for (i, eye) in x.eyes.iter().enumerate() {
        let base = x.pos + eye.base_local;
        draw.line()
            .start(base)
            .end(eye.tip_world)
            .weight(4.0)
            .color(srgba(0.80, 0.22, 0.12, 0.9));
        draw.ellipse()
            .xy(eye.tip_world)
            .radius(10.0)
            .color(srgba(0.95, 0.95, 0.85, 1.0));
        let pupil = vec2((time * 0.7 + i as f32).sin(), (time * 0.8 + i as f32).cos()) * 2.0;
        draw.ellipse()
            .xy(eye.tip_world + pupil)
            .radius(5.0)
            .color(srgba(0.05, 0.05, 0.05, 1.0));
    }
}

// ============================================================================
// BOIDS
// ============================================================================

fn update_boids(fish: &mut [Fish], predator: Vec2, predator_awake: bool, dt: f32, win: Rect) {
    // TEACH: Classic separation/alignment/cohesion.
    // We compute influences per fish by scanning neighbors.
    // O(n^2) but n is small (60).
    let neighbor_r = 120.0;
    let sep_r = 40.0;

    for i in 0..fish.len() {
        let mut sep = Vec2::ZERO;
        let mut ali = Vec2::ZERO;
        let mut coh = Vec2::ZERO;
        let mut n = 0.0;

        for j in 0..fish.len() {
            if i == j {
                continue;
            }
            let d = fish[i].pos.distance(fish[j].pos);
            if d < neighbor_r {
                n += 1.0;
                ali += fish[j].vel;
                coh += fish[j].pos;
                if d < sep_r && d > 0.0 {
                    sep += (fish[i].pos - fish[j].pos) / d;
                }
            }
        }

        let mut accel = Vec2::ZERO;
        if n > 0.0 {
            ali = (ali / n).normalize_or_zero() * 60.0;
            coh = ((coh / n) - fish[i].pos).normalize_or_zero() * 40.0;
            sep = sep.normalize_or_zero() * 90.0;
            accel += ali + coh + sep;
        }

        if predator_awake {
            let d = fish[i].pos.distance(predator);
            if d < 260.0 {
                accel += (fish[i].pos - predator).normalize_or_zero() * (260.0 - d) * 1.4;
            }
        }

        fish[i].vel += accel * dt;
        let speed = fish[i].vel.length().clamp(30.0, 140.0);
        fish[i].vel = fish[i].vel.normalize_or_zero() * speed;
    }

    for f in fish.iter_mut() {
        f.pos += f.vel * dt;
        // Wrap.
        if f.pos.x > win.right() {
            f.pos.x = win.left();
        }
        if f.pos.x < win.left() {
            f.pos.x = win.right();
        }
        if f.pos.y > win.top() {
            f.pos.y = win.bottom();
        }
        if f.pos.y < win.bottom() {
            f.pos.y = win.top();
        }
    }
}

// ============================================================================
// MATH / COLOR HELPERS
// ============================================================================

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn lerp_vec2(a: Vec2, b: Vec2, t: f32) -> Vec2 {
    vec2(lerp(a.x, b.x, t), lerp(a.y, b.y, t))
}

fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

fn ease_in_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t.powi(3)
}

fn ease_in_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

fn ocean_color(perlin: &nannou::noise::Perlin, time: f32, depth: f32) -> Rgb<f32> {
    // TEACH: Perlin noise gives organic gradients without assets.
    let n = perlin.get([time as f64 * 0.12, depth as f64 * 3.0]) as f32;
    let r = lerp(0.03, 0.12, 1.0 - depth) + n * 0.02;
    let g = lerp(0.05, 0.20, 1.0 - depth) + n * 0.02;
    let b = lerp(0.12, 0.32, 1.0 - depth) + n * 0.03;
    rgb(r, g, b)
}
