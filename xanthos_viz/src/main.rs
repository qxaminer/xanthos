use nannou::prelude::*;
use nannou_audio as audio;
use noise::{NoiseFn, Perlin};
use rand::Rng;
use std::sync::{Arc, Mutex};

fn main() {
    nannou::app(model).update(update).run();
}

// ============================================================================
// STATE & CYCLE
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
enum State {
    Dream,
    Wake,
    Climb,
    Catch,
    Sink,
}

struct Model {
    state: State,
    state_time: f32,
    depth: f32, // 0.0 = surface, 1.0 = deep
    
    // Xanthos (the crab)
    xanthos: Xanthos,
    
    // Creatures
    fish: Vec<Fish>,
    crabs: Vec<Crab>,
    berries: Vec<Berry>,
    
    // Particles
    particles: Vec<Particle>,
    bubbles: Vec<Bubble>,
    
    // Code background
    code_lines: Vec<CodeLine>,
    code_scroll: f32,
    
    // Camera
    camera_zoom: f32,
    camera_shake: Vec2,
    camera_target: Vec2,
    
    // Audio
    audio_model: Arc<Mutex<AudioModel>>,
    _audio_stream: Option<audio::Stream<Arc<Mutex<AudioModel>>>>,
    
    // Shader uniforms
    time: f32,
    perlin: Perlin,
}

struct Xanthos {
    pos: Vec2,
    vel: Vec2,
    angle: f32,
    awake: bool,
    eye_stalks: [EyeStalk; 2],
    glow_intensity: f32,
}

struct EyeStalk {
    base: Vec2,
    tip: Vec2,
    target: Vec2,
}

struct Fish {
    pos: Vec2,
    vel: Vec2,
    size: f32,
    trail: Vec<Vec2>,
    biolum: f32,
}

struct Crab {
    pos: Vec2,
    vel: Vec2,
    angle: f32,
    size: f32,
}

struct Berry {
    pos: Vec2,
    vel: Vec2,
    size: f32,
    dissolving: bool,
    dissolve_time: f32,
}

struct Particle {
    pos: Vec2,
    vel: Vec2,
    life: f32,
    max_life: f32,
    color: Rgb<f32>,
}

struct Bubble {
    pos: Vec2,
    vel: f32,
    size: f32,
    life: f32,
}

struct CodeLine {
    text: String,
    y: f32,
    alpha: f32,
}

struct AudioModel {
    depth: f32,
    catch_trigger: bool,
    morph_trigger: bool,
    phase: f32, // Track phase for audio synthesis
}

// ============================================================================
// INITIALIZATION
// ============================================================================

fn model(app: &App) -> Model {
    app.new_window()
        .size(1920, 1080)
        .view(view)
        .key_pressed(key_pressed)
        .build()
        .unwrap();
    
    let win = app.window_rect();
    
    // Initialize audio
    let audio_model = Arc::new(Mutex::new(AudioModel {
        depth: 0.0,
        catch_trigger: false,
        morph_trigger: false,
        phase: 0.0,
    }));
    let audio_stream = audio::Host::new()
        .new_output_stream(Arc::clone(&audio_model))
        .render(audio_callback)
        .channels(2)
        .sample_rate(44_100)
        .build()
        .ok();
    if let Some(ref stream) = audio_stream {
        let _ = stream.play();
    }
    
    // Initialize Xanthos
    let xanthos = Xanthos {
        pos: vec2(0.0, -win.h() * 0.3),
        vel: vec2(0.0, 0.0),
        angle: 0.0,
        awake: false,
        eye_stalks: [
            EyeStalk {
                base: vec2(-20.0, 10.0),
                tip: vec2(-20.0, 30.0),
                target: vec2(0.0, 0.0),
            },
            EyeStalk {
                base: vec2(20.0, 10.0),
                tip: vec2(20.0, 30.0),
                target: vec2(0.0, 0.0),
            },
        ],
        glow_intensity: 0.0,
    };
    
    // Initialize fish school
    let mut fish = Vec::new();
    for _ in 0..30 {
        fish.push(Fish {
            pos: vec2(
                rand::thread_rng().gen_range(-win.w()..win.w()),
                rand::thread_rng().gen_range(-win.h()..win.h()),
            ),
            vel: vec2(
                rand::thread_rng().gen_range(-1.0..1.0),
                rand::thread_rng().gen_range(-1.0..1.0),
            ),
            size: rand::thread_rng().gen_range(10.0..20.0),
            trail: Vec::new(),
            biolum: rand::thread_rng().gen_range(0.0..1.0),
        });
    }
    
    // Initialize crabs
    let mut crabs = Vec::new();
    for _ in 0..8 {
        crabs.push(Crab {
            pos: vec2(
                rand::thread_rng().gen_range(-win.w()..win.w()),
                rand::thread_rng().gen_range(-win.h()..win.h()),
            ),
            vel: vec2(
                rand::thread_rng().gen_range(-0.5..0.5),
                rand::thread_rng().gen_range(-0.5..0.5),
            ),
            angle: rand::thread_rng().gen_range(0.0..TAU),
            size: rand::thread_rng().gen_range(15.0..25.0),
        });
    }
    
    // Initialize code lines
    let code_samples = vec![
        "fn main() {",
        "    let mut world = World::new();",
        "    world.add_entity(Crab::new());",
        "    loop {",
        "        world.update();",
        "        world.render();",
        "    }",
        "}",
        "pub struct Crab {",
        "    pos: Vec2,",
        "    vel: Vec2,",
        "}",
        "impl Crab {",
        "    fn update(&mut self) {",
        "        self.pos += self.vel;",
        "    }",
        "}",
        "use nannou::prelude::*;",
        "let win = app.window_rect();",
        "draw.ellipse()",
        "    .x_y(0.0, 0.0)",
        "    .w_h(100.0, 100.0);",
    ];
    
    let mut code_lines = Vec::new();
    let mut y = -win.h() * 0.5;
    for text in code_samples.iter().cycle().take(50) {
        code_lines.push(CodeLine {
            text: text.to_string(),
            y,
            alpha: 0.5,
        });
        y += 30.0;
    }
    
    Model {
        state: State::Dream,
        state_time: 0.0,
        depth: 0.0,
        xanthos,
        fish,
        crabs,
        berries: Vec::new(),
        particles: Vec::new(),
        bubbles: Vec::new(),
        code_lines,
        code_scroll: 0.0,
        camera_zoom: 1.0,
        camera_shake: vec2(0.0, 0.0),
        camera_target: vec2(0.0, 0.0),
        audio_model,
        _audio_stream: audio_stream,
        time: 0.0,
        perlin: Perlin::new(1),
    }
}

// ============================================================================
// AUDIO
// ============================================================================

fn audio_callback(audio_model: &mut Arc<Mutex<AudioModel>>, buffer: &mut audio::Buffer<f32>) {
    let mut model = match audio_model.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };

    // Generative drone that pitch-shifts with depth
    let sample_rate = buffer.sample_rate() as f32;
    let depth_pitch = 1.0 + model.depth * 0.5; // Lower pitch at depth
    let freq = 60.0 * depth_pitch; // Base frequency
    let phase_inc = freq / sample_rate;

    for frame in buffer.frames_mut() {
        let time = model.phase;
        let sample = (time * TAU).sin() * 0.1;

        // Add harmonics for texture
        let sample2 = (time * 3.0 * TAU).sin() * 0.05;
        let sample3 = (time * 5.0 * TAU).sin() * 0.03;

        let output = sample + sample2 + sample3;

        for channel in frame {
            *channel = output;
        }

        model.phase = (model.phase + phase_inc).fract();
    }

    // Reset triggers
    model.catch_trigger = false;
    model.morph_trigger = false;
}

// ============================================================================
// UPDATE
// ============================================================================

fn update(app: &App, model: &mut Model, _update: Update) {
    let win = app.window_rect();
    let dt = 1.0 / 60.0; // Target 60fps
    model.time += dt;
    model.state_time += dt;
    
    // State machine
    match model.state {
        State::Dream => {
            model.depth = 0.0;
            model.xanthos.awake = false;
            if model.state_time > 3.0 {
                model.state = State::Wake;
                model.state_time = 0.0;
            }
        }
        State::Wake => {
            model.depth = ease_out_cubic(model.state_time / 2.0).min(0.3);
            model.xanthos.awake = true;
            if model.state_time > 2.0 {
                model.state = State::Climb;
                model.state_time = 0.0;
            }
        }
        State::Climb => {
            model.depth = ease_in_out_cubic(model.state_time / 4.0) * 0.5 + 0.3;
            if model.state_time > 4.0 {
                model.state = State::Catch;
                model.state_time = 0.0;
                // Spawn berry
                model.berries.push(Berry {
                    pos: vec2(
                        rand::thread_rng().gen_range(-100.0..100.0),
                        model.xanthos.pos.y + 200.0,
                    ),
                    vel: vec2(0.0, -50.0),
                    size: 15.0,
                    dissolving: false,
                    dissolve_time: 0.0,
                });
            }
        }
        State::Catch => {
            model.depth = 0.5;
            // Check berry catch
            for berry in &mut model.berries {
                let dist = (berry.pos - model.xanthos.pos).length();
                if dist < 50.0 && !berry.dissolving {
                    berry.dissolving = true;
                    // Create particles
                    for _ in 0..20 {
                        let angle = rand::thread_rng().gen_range(0.0..TAU);
                        let speed = rand::thread_rng().gen_range(20.0..60.0);
                        model.particles.push(Particle {
                            pos: berry.pos,
                            vel: vec2(angle.cos(), angle.sin()) * speed,
                            life: 1.0,
                            max_life: 1.0,
                            color: rgb(1.0, 0.4, 0.2),
                        });
                    }
                    // Camera shake
                    model.camera_shake = vec2(
                        rand::thread_rng().gen_range(-5.0..5.0),
                        rand::thread_rng().gen_range(-5.0..5.0),
                    );
                    // Trigger chime sound (would need to send message to audio thread)
                    // For now, we'll handle this in the audio callback via depth changes
                }
            }
            if model.state_time > 2.0 {
                model.state = State::Sink;
                model.state_time = 0.0;
            }
        }
        State::Sink => {
            model.depth = ease_in_cubic(model.state_time / 5.0) * 0.5 + 0.5;
            if model.depth >= 1.0 {
                model.state = State::Dream;
                model.state_time = 0.0;
                model.depth = 0.0;
            }
        }
    }
    
    // Update Xanthos
    let target_y = -win.h() * 0.3 + model.depth * win.h() * 0.6;
    model.xanthos.pos.y = lerp(model.xanthos.pos.y, target_y, 0.05);

    // Idle drift motion (uses vel and angle fields)
    model.xanthos.vel.x = (model.time * 0.5).sin() * 20.0;
    model.xanthos.vel.y = (model.time * 0.7).cos() * 10.0;
    model.xanthos.pos.x += model.xanthos.vel.x * dt * 0.5;
    model.xanthos.angle = (model.time * 0.3).sin() * 0.1; // Gentle rocking
    
    // Update eye stalks with IK
    let mouse = app.mouse.position();
    for stalk in &mut model.xanthos.eye_stalks {
        let base_world = model.xanthos.pos + stalk.base;
        let target = if model.xanthos.awake {
            mouse
        } else {
            base_world
        };
        stalk.target = target;
        
        // Simple IK: point towards target
        let dir = (target - base_world).normalize();
        let length = 20.0;
        stalk.tip = base_world + dir * length;
    }
    
    // Update glow
    model.xanthos.glow_intensity = if model.xanthos.awake {
        (model.time * 2.0).sin() * 0.5 + 0.5
    } else {
        0.0
    };
    
    // Update fish with boids
    for i in 0..model.fish.len() {
        let mut separation = vec2(0.0, 0.0);
        let mut alignment = vec2(0.0, 0.0);
        let mut cohesion = vec2(0.0, 0.0);
        let mut neighbors = 0;
        
        for j in 0..model.fish.len() {
            if i == j { continue; }
            let dist = (model.fish[i].pos - model.fish[j].pos).length();
            if dist < 100.0 {
                neighbors += 1;
                // Separation
                if dist > 0.0 {
                    separation += (model.fish[i].pos - model.fish[j].pos).normalize() / dist;
                }
                // Alignment
                alignment += model.fish[j].vel;
                // Cohesion
                cohesion += model.fish[j].pos;
            }
        }
        
        if neighbors > 0 {
            separation = separation.normalize() * 0.5;
            alignment = (alignment / neighbors as f32).normalize() * 0.3;
            cohesion = ((cohesion / neighbors as f32) - model.fish[i].pos).normalize() * 0.2;
        }
        
        // Avoid Xanthos if awake
        let avoid_xanthos = if model.xanthos.awake {
            let dist = (model.fish[i].pos - model.xanthos.pos).length();
            if dist < 200.0 {
                (model.fish[i].pos - model.xanthos.pos).normalize() * (1.0 - dist / 200.0) * 2.0
            } else {
                vec2(0.0, 0.0)
            }
        } else {
            vec2(0.0, 0.0)
        };
        
        model.fish[i].vel += separation + alignment + cohesion + avoid_xanthos;
        model.fish[i].vel = model.fish[i].vel.normalize() * 2.0;
        let vel = model.fish[i].vel;
        model.fish[i].pos += vel * dt * 60.0;
        
        // Wrap around
        if model.fish[i].pos.x > win.right() { model.fish[i].pos.x = win.left(); }
        if model.fish[i].pos.x < win.left() { model.fish[i].pos.x = win.right(); }
        if model.fish[i].pos.y > win.top() { model.fish[i].pos.y = win.bottom(); }
        if model.fish[i].pos.y < win.bottom() { model.fish[i].pos.y = win.top(); }
        
        // Update trail
        let pos = model.fish[i].pos;
        model.fish[i].trail.push(pos);
        if model.fish[i].trail.len() > 10 {
            model.fish[i].trail.remove(0);
        }
        
        // Bioluminescence
        model.fish[i].biolum = (model.time * 3.0 + i as f32).sin() * 0.5 + 0.5;
    }
    
    // Update crabs (scatter behavior)
    for crab in &mut model.crabs {
        // Avoid Xanthos
        let dist = (crab.pos - model.xanthos.pos).length();
        if dist < 150.0 && model.xanthos.awake {
            crab.vel = (crab.pos - model.xanthos.pos).normalize() * 1.5;
        } else {
            crab.vel = crab.vel.rotate(rand::thread_rng().gen_range(-0.1..0.1));
            crab.vel = crab.vel.normalize() * 0.8;
        }
        crab.pos += crab.vel * dt * 60.0;
        crab.angle = crab.vel.angle();
        
        // Wrap around
        if crab.pos.x > win.right() { crab.pos.x = win.left(); }
        if crab.pos.x < win.left() { crab.pos.x = win.right(); }
        if crab.pos.y > win.top() { crab.pos.y = win.bottom(); }
        if crab.pos.y < win.bottom() { crab.pos.y = win.top(); }
    }
    
    // Update berries
    for berry in &mut model.berries {
        berry.pos += berry.vel * dt;
        if berry.dissolving {
            berry.dissolve_time += dt;
            berry.size *= 0.95;
        }
    }
    model.berries.retain(|b| b.size > 1.0);
    
    // Update particles
    for particle in &mut model.particles {
        particle.pos += particle.vel * dt;
        particle.vel *= 0.98; // Friction
        particle.life -= dt * 2.0;
    }
    model.particles.retain(|p| p.life > 0.0);
    
    // Update bubbles (pressure = depth)
    if rand::thread_rng().gen::<f32>() < model.depth * 0.1 {
        model.bubbles.push(Bubble {
            pos: vec2(
                rand::thread_rng().gen_range(-win.w()..win.w()),
                win.bottom() - 50.0,
            ),
            vel: rand::thread_rng().gen_range(30.0..60.0),
            size: rand::thread_rng().gen_range(5.0..15.0),
            life: 1.0,
        });
    }
    for bubble in &mut model.bubbles {
        bubble.pos.y += bubble.vel * dt;
        bubble.life -= dt * 0.5;
    }
    model.bubbles.retain(|b| b.life > 0.0 && b.pos.y < win.top());
    
    // Update code scroll
    model.code_scroll += dt * 50.0;
    for line in &mut model.code_lines {
        line.y += dt * 50.0;
        if line.y > win.top() + 50.0 {
            line.y = win.bottom() - 50.0;
        }
        // Parallax effect
        line.alpha = (0.3 + model.depth * 0.4) * (1.0 - (line.y.abs() / win.h()));
    }
    
    // Update camera
    model.camera_zoom = 1.0 + model.depth * 0.3;
    model.camera_shake *= 0.9;
    model.camera_target = model.xanthos.pos;
    
    // Update audio model depth
    if let Ok(mut audio) = model.audio_model.lock() {
        audio.depth = model.depth;
    }
}

// ============================================================================
// RENDERING
// ============================================================================

fn view(app: &App, model: &Model, frame: Frame) {
    let draw = app.draw();
    let win = app.window_rect();
    
    // Clear with ocean gradient (Perlin noise)
    let bg_color = ocean_gradient(model.depth, &model.perlin, model.time);
    draw.rect()
        .wh(win.wh())
        .xy(win.xy())
        .color(bg_color);
    
    // Apply camera transform
    let camera_offset = model.camera_target + model.camera_shake;
    let scale = model.camera_zoom;
    
    // Draw code background (parallax layer) with chromatic aberration
    draw.scale(1.0 / scale).translate(vec3(-camera_offset.x * 0.1, -camera_offset.y * 0.1, 0.0));
    for line in &model.code_lines {
        // Chromatic aberration: separate RGB channels with depth-based offset
        let offset = model.depth * 4.0;

        // Red channel (offset left)
        draw.text(&line.text)
            .x_y(-offset, line.y)
            .font_size(16)
            .color(rgba(0.8, 0.0, 0.0, line.alpha * 0.4))
            .left_justify();

        // Green channel (center)
        draw.text(&line.text)
            .x_y(0.0, line.y)
            .font_size(16)
            .color(rgba(0.0, 0.7, 0.0, line.alpha * 0.5))
            .left_justify();

        // Blue channel (offset right)
        draw.text(&line.text)
            .x_y(offset, line.y)
            .font_size(16)
            .color(rgba(0.0, 0.0, 1.0, line.alpha * 0.4))
            .left_justify();

        // Main cyan text (composite)
        draw.text(&line.text)
            .x_y(0.0, line.y)
            .font_size(16)
            .color(rgba(0.2, 0.6, 0.8, line.alpha * 0.6))
            .left_justify();
    }
    draw.reset();
    
    // Main scene (with camera)
    draw.scale(scale).translate(vec3(-camera_offset.x, -camera_offset.y, 0.0));
    
    // Draw bubbles
    for bubble in &model.bubbles {
        draw.ellipse()
            .xy(bubble.pos)
            .radius(bubble.size)
            .color(rgba(0.7, 0.9, 1.0, bubble.life * 0.3));
    }
    
    // Draw fish with bioluminescent trails
    for fish in &model.fish {
        // Trail
        for (i, &point) in fish.trail.iter().enumerate() {
            let alpha = (i as f32 / fish.trail.len() as f32) * fish.biolum * 0.5;
            draw.ellipse()
                .xy(point)
                .radius(2.0)
                .color(rgba(0.2, 0.8, 1.0, alpha));
        }
        // Fish body
        draw.ellipse()
            .xy(fish.pos)
            .radius(fish.size)
            .color(rgba(0.3, 0.7, 1.0, 0.8));
    }
    
    // Draw crabs
    for crab in &model.crabs {
        draw.ellipse()
            .xy(crab.pos)
            .radius(crab.size)
            .rotate(crab.angle)
            .color(rgba(0.6, 0.4, 0.2, 0.7));
    }
    
    // Draw berries
    for berry in &model.berries {
        draw.ellipse()
            .xy(berry.pos)
            .radius(berry.size)
            .color(rgba(1.0, 0.4, 0.2, 1.0 - berry.dissolve_time));
    }
    
    // Draw particles
    for particle in &model.particles {
        let alpha = particle.life / particle.max_life;
        draw.ellipse()
            .xy(particle.pos)
            .radius(3.0)
            .color(rgba(particle.color.red, particle.color.green, particle.color.blue, alpha));
    }
    
    // Draw Xanthos with radial glow
    let glow_radius = 50.0 + model.xanthos.glow_intensity * 30.0;
    draw.ellipse()
        .xy(model.xanthos.pos)
        .radius(glow_radius)
        .rotate(model.xanthos.angle)
        .color(rgba(1.0, 0.6, 0.2, model.xanthos.glow_intensity * 0.3));

    // Draw Xanthos body (slightly wider than tall for crab shape)
    draw.ellipse()
        .xy(model.xanthos.pos)
        .w_h(70.0, 50.0)
        .rotate(model.xanthos.angle)
        .color(rgb(0.8, 0.3, 0.1));

    // Draw eye stalks with IK (rotated with body)
    let cos_a = model.xanthos.angle.cos();
    let sin_a = model.xanthos.angle.sin();
    for stalk in &model.xanthos.eye_stalks {
        // Rotate base position by body angle
        let rotated_base = vec2(
            stalk.base.x * cos_a - stalk.base.y * sin_a,
            stalk.base.x * sin_a + stalk.base.y * cos_a,
        );
        let base_world = model.xanthos.pos + rotated_base;
        draw.line()
            .start(base_world)
            .end(stalk.tip)
            .weight(3.0)
            .color(rgb(0.6, 0.2, 0.1));
        draw.ellipse()
            .xy(stalk.tip)
            .radius(5.0)
            .color(rgb(1.0, 0.0, 0.0));
    }
    
    draw.to_frame(app, &frame).unwrap();
}

// ============================================================================
// HELPERS
// ============================================================================

fn ocean_gradient(depth: f32, perlin: &Perlin, time: f32) -> Rgb<f32> {
    let noise_val = perlin.get([time as f64 * 0.1, depth as f64 * 10.0]) as f32;
    let r = lerp(0.05, 0.15, depth) + noise_val * 0.05;
    let g = lerp(0.1, 0.3, depth) + noise_val * 0.05;
    let b = lerp(0.3, 0.5, depth) + noise_val * 0.05;
    rgb(r, g, b)
}

fn ease_out_cubic(t: f32) -> f32 {
    let t = t.min(1.0);
    1.0 - (1.0 - t).powf(3.0)
}

fn ease_in_cubic(t: f32) -> f32 {
    t.min(1.0).powf(3.0)
}

fn ease_in_out_cubic(t: f32) -> f32 {
    let t = t.min(1.0);
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powf(3.0) / 2.0
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn key_pressed(_app: &App, _model: &mut Model, key: Key) {
    match key {
        Key::Space => {
            // Reset cycle
        }
        _ => {}
    }
}
