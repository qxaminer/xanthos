# Xanthos Visualization - Hello Universe

A deep sea documentary × demoscene × poetry meditation for devs who dream in arrays.

## Features

### 🌊 PARTICLES
- **Fish trail bioluminescence**: Glowing trails follow each fish
- **Berry particle dispersion**: Berries dissolve into particle explosions on catch
- **Depth pressure bubbles**: Rising bubbles increase with depth

### 🐟 BOIDS
- **Real separation/alignment/cohesion**: Proper boids algorithm for fish schooling
- **Fish school behavior**: Fish form natural schools
- **Crabs scatter**: Crabs flee from Xanthos when awake
- **Xanthos predator**: When awake, Xanthos causes fish and crabs to avoid

### 🎨 SHADERS
- **Perlin noise ocean gradient**: Procedural ocean colors that shift with depth
- **Chromatic aberration on code**: Code text has depth-based color shifting
- **Radial glow on Xanthos**: Pulsing glow when Xanthos is awake

### 🔊 SOUND
- **Generative drones**: Pitch-shift with depth (lower at deeper depths)
- **Chime on berry catch**: Audio feedback on successful catch
- **Granular texture**: Harmonic layers for rich soundscape

### 📷 CAMERA
- **Depth-based zoom**: Camera zooms in as depth increases
- **Parallax layers**: Code background moves at different speed than creatures
- **Subtle shake on catch**: Camera shakes when berry is caught

### ✨ POLISH
- **60fps target**: Optimized for smooth performance
- **Easing functions**: Cubic easing instead of linear interpolation
- **IK eye stalks**: Inverse kinematics for natural eye movement

## Cycle

The visualization follows a dream→wake→climb→catch→sink cycle:

1. **Dream**: Xanthos sleeps at surface (depth = 0.0)
2. **Wake**: Xanthos awakens and begins descent
3. **Climb**: Xanthos climbs toward a berry
4. **Catch**: Berry is caught, particles explode, camera shakes
5. **Sink**: Xanthos sinks back down, cycle repeats

## Running

```bash
cd xanthos_viz
cargo run --release
```

## Requirements

- Rust (2021 edition)
- M4 Mac (optimized for Apple Silicon)
- Nannou 0.20
- nannou_audio 0.7
- noise 0.9
- rand 0.8

## Controls

- **Space**: Reset cycle (placeholder for future interaction)

## Technical Notes

- Single-file architecture for simplicity
- No external assets required
- Procedural generation for all visuals
- GPU-optimized rendering via Nannou/wgpu
