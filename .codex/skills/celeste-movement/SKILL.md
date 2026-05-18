---
name: celeste-movement
description: Specify, review, and implement player movement behavior for this Bevy Celeste-like project, especially climb, wall interaction, jump, dash, and state transition rules. Use this when the user wants movement mechanics documented, compared against code, debugged, or extended.
metadata:
  short-description: Document and debug Celeste-like movement
---

# Celeste-like Movement

Use this skill when the user wants to define movement rules, compare intended behavior against the current implementation, debug movement bugs, or add a new movement mechanic for this project.

This skill is for player verbs and state rules, not room layout. For room and chapter design, use `celeste-level-design`.

## Source Of Truth

Before making claims about behavior, anchor to the current code:

- Movement tuning lives in `src/constants.rs`
- Movement state and transitions live in `src/components.rs`
- Input, state changes, physics, and collision resolution live in `src/systems/player.rs`
- Visual climb feedback lives in `src/systems/animation.rs`

If code and user description disagree, do not silently pick one. Call out the mismatch first.

## Current Movement Scope

As of the current project state, the codebase includes:

- ground acceleration, friction, and turn handling
- coyote time and jump buffer
- variable jump height
- wall slide
- climb state
- wall jump / wall kick variants
- dash, dash freeze, and dash end handling
- dash corner correction
- downward dash slide into grounded super jump
- crouch collider swapping

Do not document or design around mechanics that are not implemented unless the user explicitly wants a forward-looking spec.

## State Model

Current player states:

- `Normal`
- `Climb`
- `Dash`

When reviewing or editing movement, reason in this order:

1. Entry conditions
2. Sustain conditions
3. Exit conditions
4. Velocity applied while in state
5. Collision-side corrections
6. Interactions with jump, dash, crouch, and timers

## Confirmed Design Rules

These rules are confirmed by the user and should be treated as intended behavior unless later revised.

### Climb Entry

Enter climb when all are true:

- the player is near a wall
- the player is facing that wall
- the grab button is held

### Climb Sustain

Remain in climb while the grab button remains held.

Important:
- this is intentionally not identical to the entry rule
- entry requires facing the wall
- sustain currently does not need to keep re-checking facing direction as a design rule

### Climb Exit

Exit climb when any of these happens:

- the player releases grab
- the player reaches the top of the wall and performs an automatic top-out onto the platform
- the player jumps away from the wall
- the player enters dash

### Climb Top-Out

Top-out is intended behavior and should exist even if not yet implemented.

Expected behavior:

- when the player climbs past the top edge of a wall and there is valid standing space on top
- the character automatically advances a short distance toward the wall/platform side
- the final position should place the player cleanly standing on top of the ledge
- this transition exits `Climb`

Treat this as a dedicated ledge-climb rule, not just generic collision correction.

### Climb Jump

While in climb, pressing jump has two branches:

- If the player is pressing away from the wall, perform an away jump / wall kick and exit climb.
- Otherwise, perform an upward climb jump and exit climb.

For the upward climb jump:

- its vertical launch should match the ground jump behavior
- holding jump should produce the same jump-height curve as a normal ground jump
- keep the existing climb-jump parameter and meaning in `src/constants.rs`, but its value should match the intended behavior
- the feel target is closer to original Celeste than the current implementation

### Dash Interaction

Dash should interrupt climb immediately.

If dash begins, the player leaves `Climb` and enters `Dash` in the same action flow.

## Known Intentional Differences Vs Current Code

These are known and should not be treated as accidental unless the user changes direction later.

- Climb entry and climb sustain are intentionally asymmetric.
- The project wants a dedicated climb top-out behavior, but it has not been implemented yet.
- The climb jump tuning parameter should remain semantically distinct even if its numeric value becomes equal to normal jump launch.

## Known Bugs Or Required Corrections

These are already confirmed by the user as bugs or required fixes.

- Releasing grab should exit climb.
- Climb top-out should be added.
- The current climb jump height is too low relative to ground jump.
- The current non-away climb jump feel is not close enough to Celeste and should be improved.

## Pending Validation

One behavior is still intentionally left open until runtime testing:

- if the player is somehow still in `Climb` after wall contact is lost, and jump is pressed, current code may consume the jump buffer without applying a jump

Do not write this up as intended design. Treat it as a runtime verification point and confirm with the user after testing.

## Review Workflow

When the user asks for movement review or debugging:

1. Read `src/constants.rs`, `src/components.rs`, and `src/systems/player.rs`.
2. Extract the exact entry, sustain, and exit conditions for the relevant state.
3. Compare them against the intended mechanic.
4. List mismatches before changing design assumptions.
5. Only then propose or implement fixes.

When the user asks for a new movement spec:

1. Separate confirmed mechanics from planned mechanics.
2. Mark anything not yet implemented.
3. Preserve parameter meaning if the user asks to retune without renaming semantics.
4. Note any risky edge cases that need runtime validation.

## Implementation Guidance

Prefer small, explicit movement rules over broad magic corrections.

- Put tuning values in `src/constants.rs`.
- Keep state transitions readable in `src/systems/player.rs`.
- If a mechanic has special collision behavior, name it explicitly instead of burying it inside generic motion code.
- When matching Celeste-like feel, prioritize state timing and launch vectors before adding extra complexity.

For climb-related work, verify all of these together:

- wall detection
- facing checks
- grab input timing
- state transition order in `FixedUpdate`
- climb jump launch values
- ledge top-out placement
- dash interruption behavior

## Output Style

When using this skill in a response:

- distinguish clearly between intended behavior, current behavior, and planned behavior
- call out exact files that likely own the behavior
- if behavior is still ambiguous, ask whether it is a bug or an intended hidden mechanic before documenting it as final
