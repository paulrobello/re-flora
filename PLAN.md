# Butterfly Movement Refactor Plan

## Goal

Refactor butterfly behavior only (leave all other particle types untouched) so that:

1. Initial spawn picks random `x/z` in map range and derives `y` from terrain query.
2. Each update chooses movement using a 3D perlin worm direction.
3. Before moving from `P` to `N`, cast a terrain ray from `P` along `D`.
4. If terrain is hit before `dist(P, N)`, retry with a new perlin-worm direction.
5. Border crossing is no longer immediate despawn; it triggers retry.
6. Butterfly is despawned only when:
   - retry count exceeds 3 in one update cycle, or
   - lifetime expires.

## Scope Constraints

- Butterfly logic only.
- No behavior changes for leaves or any non-butterfly particles.
- Keep existing renderer and animation behavior unless required by movement rewrite.

## Files To Update

- `src/particles/emitters.rs`
- `src/app/core/particles.rs`
- `src/particles/system.rs` (only butterfly-specific despawn condition handling)

## Design Decisions

- Use full 3D distance for `dist(P, N)`.
- Retry budget per update: max 3 retries (4 total attempts including first).
- Forward ray origin: `P` with a small upward epsilon to avoid immediate self-hit artifacts.
- If terrain ray is invalid/no-hit, treat as unblocked (allow attempt), unless border check fails.
- Border check is performed against candidate `N`.

## Implementation Steps

### 1. Emitter spawn rewrite (butterfly-only)

- Store emitter map bounds data needed for uniform map `x/z` sampling.
- Replace radial/wander spawn position with random map-range `x/z`.
- Keep spawn handle tracking unchanged.

### 2. Initial terrain-based height assignment

- In app-side butterfly control flow, query terrain for newly spawned butterfly `x/z`.
- Set initial `y = terrain_height + chosen_height_offset`.
- Handle invalid height samples safely (fallback/retry path, no panic).

### 3. Per-update worm guidance pipeline

- Add butterfly worm-direction generation (3D perlin worm) that can produce a new candidate direction each retry.
- For each butterfly each update:
  - Compute candidate `D`.
  - Compute candidate `N = P + D * step_len`.
  - If `N` is outside map bounds => retry.
  - Cast terrain ray from `P` along `D`.
  - If hit distance `< dist(P, N)` => retry.
  - On success, commit movement intent (velocity/position target for this frame).

### 4. Retry + despawn policy

- If retries exceed 3 for that butterfly in that update => despawn.
- Remove border-immediate-despawn behavior.
- Remove ground-proximity despawn behavior.

### 5. Trim old butterfly movement behaviors

- Remove/disable old butterfly-specific wander/home-steer/bob-ground constraint path from active update flow.
- Keep leaf and shared falling behavior untouched.

### 6. Lifetime-only + retry-limit despawn correctness

- Ensure butterflies are not killed by generic non-butterfly rules (`y < 0` fallback etc.) unless explicitly intended.
- Keep normal lifetime expiry behavior.

## Validation Plan

1. **Compile check** - Build project and fix compile errors introduced by API/flow changes.

2. **Behavior checks (runtime)**
   - Butterflies spawn across map `x/z` distribution.
   - Spawn height follows terrain.
   - Butterflies try alternative directions near obstacles/borders instead of immediate despawn.
   - Butterflies despawn after >3 failed retries in one update.
   - Butterflies still despawn on lifetime timeout.
   - Leaves behave exactly as before.

3. **Logging (temporary during dev)**
   - Count retries and retry-limit despawns.
   - Track blocked-ray events and border-retry events.
   - Remove or reduce noisy logs after validation.

## Risks / Watchouts

- Terrain ray batching vs per-butterfly retries may need careful control-flow to avoid performance spikes.
- Retry loops must remain bounded and deterministic.
- Must avoid accidentally changing global particle update semantics for non-butterflies.

## Done Criteria

- All requested butterfly behaviors are implemented.
- No behavior regressions for non-butterfly particle types.
- Build succeeds and runtime checks pass.
