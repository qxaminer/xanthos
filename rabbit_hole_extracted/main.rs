// xanthos.rs
// "The crab dreams of arrays before arrays existed"
//
// cargo run --release

use nannou::prelude::*;

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

// The pattern was always there in the swirl
// each fish a vertex
// each turn a transformation matrix
// each predator-shadow a shader uniform
// passed down from the sun

struct Crab {
    pos: vec2<f32>,
    eyes: vec2<f32>,
    dream: f32,
}

// Blueberry muffins dissolve in the deep
// That's why Xanthos went up top
// to catch some berry

@fragment
fn dissolve(uv: vec2<f32>, depth: f32) -> vec4<f32> {
    let crumb = noise(uv * 10.0);
    let pressure = exp(-depth * 0.1);
    return vec4(crumb * pressure, 0.0, crumb, pressure);
}

// The lobster doesn't dream of arrays
// The lobster is the array now
// Segmented. Indexed. Accessed by fork.

fn rabbit_hole(depth: f32) -> f32 {
    return 38.83;  // miles, as the crow flies
}

// no rabbits
// no men
// no words
// just core
"#;

fn main() {
    nannou::app(model).update(update).run();
}

struct Fish {
    pos: Vec2,
    vel: Vec2,
    becoming_crab: f32,  // 0.0 = fish, 1.0 = baby crab
}

struct Xanthos {
    pos: Vec2,
    eye_angles: (f32, f32),  // lidless, always watching
    depth: f32,
    dreaming: bool,
    has_berry: bool,
}

struct Model {
    fish: Vec<Fish>,
    xanthos: Xanthos,
    code_scroll: f32,
    alpha_phase: f32,
    time: f32,
    berry_pos: Option<Vec2>,
}

fn model(app: &App) -> Model {
    app.new_window()
        .size(1200, 800)
        .title("Xanthos Dreams of Arrays")
        .view(view)
        .build()
        .unwrap();

    // Initialize fish swirl
    let fish = (0..120)
        .map(|i| {
            let angle = i as f32 * 0.15;
            let r = 100.0 + (i as f32 * 0.5);
            Fish {
                pos: vec2(angle.cos() * r, angle.sin() * r - 100.0),
                vel: vec2(0.0, 0.0),
                becoming_crab: 0.0,
            }
        })
        .collect();

    // Xanthos starts in the deep
    let xanthos = Xanthos {
        pos: vec2(0.0, -300.0),
        eye_angles: (0.0, 0.0),
        depth: 1.0,
        dreaming: true,
        has_berry: false,
    };

    Model {
        fish,
        xanthos,
        code_scroll: 0.0,
        alpha_phase: 0.0,
        time: 0.0,
        berry_pos: None,
    }
}

fn update(_app: &App, model: &mut Model, _update: Update) {
    model.time += 0.016;
    model.code_scroll += 0.5;
    model.alpha_phase += 0.02;

    // Berry spawns occasionally at surface
    if model.berry_pos.is_none() && random_f32() < 0.005 {
        model.berry_pos = Some(vec2(random_range(-400.0, 400.0), 350.0));
    }

    // Berry dissolves (falls) if not caught
    if let Some(ref mut berry) = model.berry_pos {
        berry.y -= 0.3;
        if berry.y < -400.0 {
            model.berry_pos = None;  // dissolved in the deep
        }
    }

    // Xanthos behavior
    if model.xanthos.dreaming {
        // Dream phase: watch the swirl
        model.xanthos.eye_angles.0 += 0.03;
        model.xanthos.eye_angles.1 -= 0.02;

        // Wake up after a while
        if model.time > 8.0 && model.time < 8.1 {
            model.xanthos.dreaming = false;
        }
    } else {
        // Climbing phase: go for berry
        if let Some(berry) = &model.berry_pos {
            let dir = (*berry - model.xanthos.pos).normalize();
            model.xanthos.pos += dir * 2.0;
            model.xanthos.depth = map_range(model.xanthos.pos.y, -300.0, 350.0, 1.0, 0.0);

            // Eyes track berry
            let to_berry = *berry - model.xanthos.pos;
            model.xanthos.eye_angles.0 = to_berry.y.atan2(to_berry.x);
            model.xanthos.eye_angles.1 = model.xanthos.eye_angles.0 + 0.3;

            // Catch berry
            if model.xanthos.pos.distance(*berry) < 30.0 {
                model.xanthos.has_berry = true;
                model.berry_pos = None;
            }
        } else if !model.xanthos.has_berry {
            // No berry, sink back to dream
            model.xanthos.pos.y -= 0.5;
            if model.xanthos.pos.y < -280.0 {
                model.xanthos.dreaming = true;
                model.xanthos.pos.y = -300.0;
            }
        }
    }

    // Fish swirl (boid-like behavior)
    let center = vec2(0.0, -50.0 + (model.time * 0.5).sin() * 50.0);
    
    for fish in &mut model.fish {
        // Swirl toward center
        let to_center = center - fish.pos;
        let dist = to_center.length();
        
        // Tangential velocity (creates swirl)
        let tangent = vec2(-to_center.y, to_center.x).normalize();
        
        // Combine: orbit + slight inward pull
        let orbit_speed = 2.0 / (1.0 + dist * 0.01);
        fish.vel = tangent * orbit_speed + to_center.normalize() * 0.1;
        
        // Add some chaos
        fish.vel.x += (model.time * 3.0 + fish.pos.y * 0.1).sin() * 0.3;
        fish.vel.y += (model.time * 2.7 + fish.pos.x * 0.1).cos() * 0.3;
        
        fish.pos += fish.vel;

        // Fish become crabs during the transformation phase
        if model.time > 5.0 && model.time < 12.0 {
            fish.becoming_crab = ((model.time - 5.0) / 7.0).min(1.0);
        }
        if model.time > 15.0 {
            fish.becoming_crab = (1.0 - (model.time - 15.0) / 3.0).max(0.0);
        }
    }
}

fn view(app: &App, model: &Model, frame: Frame) {
    let draw = app.draw();
    let win = app.window_rect();

    // Deep dark gradient background
    draw.background().color(BLACK);
    
    // Depth gradient (darker at bottom)
    for i in 0..20 {
        let y = map_range(i, 0, 19, win.bottom(), win.top());
        let h = win.h() / 20.0;
        let darkness = map_range(i as f32, 0.0, 19.0, 0.0, 0.15);
        draw.rect()
            .x_y(0.0, y)
            .w_h(win.w(), h)
            .color(srgba(0.0, 0.02, 0.08, darkness));
    }

    // === SCROLLING CODE BACKGROUND ===
    let alpha_fluctuate = (model.alpha_phase.sin() * 0.5 + 0.5) * 0.15 + 0.05;
    let lines: Vec<&str> = CODE.lines().collect();
    let line_height = 18.0;
    let total_height = lines.len() as f32 * line_height;
    let scroll_offset = model.code_scroll % total_height;

    for (i, line) in lines.iter().enumerate() {
        let y = win.top() - 50.0 - (i as f32 * line_height) + scroll_offset;
        
        // Wrap around
        let y = if y > win.top() + 50.0 {
            y - total_height
        } else if y < win.bottom() - 50.0 {
            y + total_height
        } else {
            y
        };

        // Fluctuating alpha based on position and time
        let local_alpha = alpha_fluctuate * (1.0 + (y * 0.01 + model.alpha_phase * 2.0).sin() * 0.5);
        
        draw.text(line)
            .x_y(-200.0, y)
            .font_size(14)
            .left_justify()
            .color(srgba(0.3, 0.8, 0.4, local_alpha));
    }

    // === FISH / BABY CRABS SWIRL ===
    for fish in &model.fish {
        let t = fish.becoming_crab;
        
        if t < 0.5 {
            // Draw as fish
            let fish_alpha = 1.0 - t * 2.0;
            let angle = fish.vel.y.atan2(fish.vel.x);
            
            // Fish body
            draw.ellipse()
                .x_y(fish.pos.x, fish.pos.y)
                .w_h(12.0, 6.0)
                .rotate(angle)
                .color(srgba(0.7, 0.75, 0.9, fish_alpha * 0.8));
            
            // Fish tail
            let tail_offset = vec2(-8.0, 0.0).rotate(angle);
            draw.tri()
                .points(
                    fish.pos + tail_offset,
                    fish.pos + tail_offset + vec2(-6.0, 4.0).rotate(angle),
                    fish.pos + tail_offset + vec2(-6.0, -4.0).rotate(angle),
                )
                .color(srgba(0.6, 0.65, 0.85, fish_alpha * 0.6));
        }
        
        if t > 0.3 {
            // Draw as baby crab (fading in)
            let crab_alpha = ((t - 0.3) / 0.7).min(1.0);
            draw_baby_crab(&draw, fish.pos, crab_alpha, model.time);
        }
    }

    // === XANTHOS (the dreamer) ===
    draw_xanthos(&draw, &model.xanthos, model.time);

    // === BERRY (if present) ===
    if let Some(berry) = model.berry_pos {
        // Blueberry with dissolving edges at depth
        let depth_factor = map_range(berry.y, -400.0, 350.0, 0.3, 1.0);
        
        draw.ellipse()
            .x_y(berry.x, berry.y)
            .w_h(16.0 * depth_factor, 14.0 * depth_factor)
            .color(srgba(0.2, 0.1, 0.6, depth_factor));
        
        // Berry highlight
        draw.ellipse()
            .x_y(berry.x - 3.0, berry.y + 3.0)
            .w_h(4.0 * depth_factor, 3.0 * depth_factor)
            .color(srgba(0.5, 0.4, 0.9, depth_factor * 0.5));

        // Dissolving particles if deep
        if berry.y < 0.0 {
            for i in 0..5 {
                let offset = vec2(
                    (model.time * 2.0 + i as f32).sin() * 20.0,
                    (model.time * 1.5 + i as f32 * 0.7).cos() * 15.0 - 10.0,
                );
                draw.ellipse()
                    .x_y(berry.x + offset.x, berry.y + offset.y)
                    .w_h(3.0, 3.0)
                    .color(srgba(0.3, 0.2, 0.5, (1.0 - depth_factor) * 0.3));
            }
        }
    }

    // === TITLE ===
    draw.text("Xanthos Dreams of Arrays")
        .x_y(0.0, win.top() - 30.0)
        .font_size(24)
        .color(srgba(0.9, 0.85, 0.7, 0.7));

    // === STATE TEXT ===
    let state_text = if model.xanthos.has_berry {
        "// this is the moment, before dissolution"
    } else if model.xanthos.dreaming {
        "// the gonads know, the gonads always knew"
    } else {
        "// Xanthos went up top to catch some berry"
    };
    
    draw.text(state_text)
        .x_y(0.0, win.bottom() + 30.0)
        .font_size(16)
        .color(srgba(0.5, 0.8, 0.5, 0.6));

    draw.to_frame(app, &frame).unwrap();
}

fn draw_baby_crab(draw: &Draw, pos: Vec2, alpha: f32, time: f32) {
    let wobble = (time * 5.0 + pos.x * 0.1).sin() * 0.1;
    
    // Body
    draw.ellipse()
        .x_y(pos.x, pos.y)
        .w_h(10.0, 8.0)
        .rotate(wobble)
        .color(srgba(0.9, 0.4, 0.3, alpha * 0.8));
    
    // Eyes (lidless!)
    for side in [-1.0, 1.0] {
        let eye_pos = pos + vec2(side * 4.0, 4.0);
        
        // Eye stalk
        draw.line()
            .start(pos + vec2(side * 3.0, 2.0))
            .end(eye_pos)
            .weight(1.5)
            .color(srgba(0.8, 0.35, 0.25, alpha));
        
        // Eye
        draw.ellipse()
            .x_y(eye_pos.x, eye_pos.y)
            .w_h(3.0, 3.0)
            .color(srgba(0.1, 0.1, 0.1, alpha));
    }
    
    // Tiny claws
    for side in [-1.0, 1.0] {
        let claw_base = pos + vec2(side * 6.0, 0.0);
        draw.ellipse()
            .x_y(claw_base.x + side * 2.0, claw_base.y)
            .w_h(4.0, 3.0)
            .color(srgba(0.85, 0.35, 0.25, alpha * 0.7));
    }
}

fn draw_xanthos(draw: &Draw, xanthos: &Xanthos, time: f32) {
    let pos = xanthos.pos;
    let size_factor = if xanthos.dreaming { 1.0 } else { 1.0 + xanthos.depth * 0.2 };
    
    // Glow when has berry
    if xanthos.has_berry {
        draw.ellipse()
            .x_y(pos.x, pos.y)
            .w_h(80.0, 60.0)
            .color(srgba(0.4, 0.2, 0.6, 0.2 + (time * 3.0).sin() * 0.1));
    }
    
    // Main body
    draw.ellipse()
        .x_y(pos.x, pos.y)
        .w_h(50.0 * size_factor, 35.0 * size_factor)
        .color(srgba(0.85, 0.25, 0.15, 0.9));
    
    // Shell pattern
    draw.ellipse()
        .x_y(pos.x, pos.y + 5.0)
        .w_h(35.0 * size_factor, 20.0 * size_factor)
        .color(srgba(0.75, 0.2, 0.1, 0.7));
    
    // Eye stalks and eyes (LIDLESS - always watching)
    for (i, side) in [-1.0_f32, 1.0].iter().enumerate() {
        let angle = if i == 0 { xanthos.eye_angles.0 } else { xanthos.eye_angles.1 };
        let stalk_length = 20.0 * size_factor;
        
        let stalk_end = pos + vec2(
            side * 15.0 + angle.cos() * stalk_length * 0.3,
            15.0 + angle.sin().abs() * stalk_length * 0.5,
        );
        
        // Stalk
        draw.line()
            .start(pos + vec2(side * 12.0, 8.0))
            .end(stalk_end)
            .weight(4.0)
            .color(srgba(0.8, 0.22, 0.12, 0.9));
        
        // Eye (NO LID)
        draw.ellipse()
            .x_y(stalk_end.x, stalk_end.y)
            .w_h(10.0, 10.0)
            .color(srgba(0.95, 0.95, 0.85, 1.0));
        
        // Pupil (tracks... something)
        let pupil_offset = vec2(angle.cos() * 2.0, angle.sin() * 2.0);
        draw.ellipse()
            .x_y(stalk_end.x + pupil_offset.x, stalk_end.y + pupil_offset.y)
            .w_h(5.0, 5.0)
            .color(srgba(0.05, 0.05, 0.05, 1.0));
    }
    
    // Claws
    for side in [-1.0, 1.0] {
        let claw_base = pos + vec2(side * 30.0, -5.0);
        let claw_angle = side * 0.3 + (time * 2.0).sin() * 0.1;
        
        // Upper claw
        draw.ellipse()
            .x_y(claw_base.x + side * 12.0, claw_base.y + 5.0)
            .w_h(18.0 * size_factor, 10.0 * size_factor)
            .rotate(claw_angle)
            .color(srgba(0.9, 0.3, 0.2, 0.85));
        
        // Lower claw
        draw.ellipse()
            .x_y(claw_base.x + side * 10.0, claw_base.y - 3.0)
            .w_h(14.0 * size_factor, 7.0 * size_factor)
            .rotate(-claw_angle * 0.5)
            .color(srgba(0.85, 0.28, 0.18, 0.8));
    }
    
    // Legs
    for i in 0..4 {
        for side in [-1.0, 1.0] {
            let leg_base = pos + vec2(side * (15.0 + i as f32 * 5.0), -10.0 - i as f32 * 3.0);
            let leg_angle = side * (0.5 + i as f32 * 0.15) + (time * 3.0 + i as f32).sin() * 0.1;
            let leg_end = leg_base + vec2(leg_angle.cos() * 15.0 * side, leg_angle.sin() * -15.0);
            
            draw.line()
                .start(leg_base)
                .end(leg_end)
                .weight(3.0)
                .color(srgba(0.75, 0.2, 0.12, 0.7));
        }
    }

    // Berry in claw if caught
    if xanthos.has_berry {
        let berry_pos = pos + vec2(35.0, 5.0);
        draw.ellipse()
            .x_y(berry_pos.x, berry_pos.y)
            .w_h(14.0, 12.0)
            .color(srgba(0.25, 0.15, 0.65, 0.95));
        draw.ellipse()
            .x_y(berry_pos.x - 3.0, berry_pos.y + 3.0)
            .w_h(4.0, 3.0)
            .color(srgba(0.5, 0.4, 0.9, 0.5));
    }
}
