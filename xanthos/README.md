# Xanthos Universe (Teaching Edition)

Nannou + Rust learning project. M4 Mac friendly.  
**Vibe:** deep sea documentary × demoscene × poetry.

## What this teaches (on purpose)

- **Ownership & borrowing** (`Vec<T>` owns, `&T` borrows, `&mut T` is exclusive)
- **Slices** (`&[T]`, `&mut [T]`) as “views” into contiguous memory
- **Iterators** (`iter_mut`, `enumerate`, `fold`)
- **Concurrency** with `Arc<Mutex<T>>` across render + audio threads
- **Bytemuck** (`Pod`, `Zeroable`, `cast_slice_mut`) for safe buffer casting

## Run

```bash
cd xanthos
cargo run --release
```

If your machine says no default toolchain is configured:

```bash
rustup default stable
```

## Audio design

- **Drone**: deeper = louder + slightly different pitch
- **Shimmer**: brighter near surface (FM-ish modulation)
- **Chime**: triggers on berry catch (decays exponentially)
- **Grains**: triggers on fish→crab transformation phase
