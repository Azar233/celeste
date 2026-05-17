---
name: celeste-level-design
description: Design rooms, scene flow, and graybox maps for this Bevy Celeste-like project based on the currently implemented movement kit.
metadata:
  short-description: Design Celeste-like rooms from player movement
---

# Celeste-like Level Design

Use this skill when the user wants to design scenes, rooms, graybox maps, or the next level-building workflow for this project.

This repository already has a strong movement prototype. The goal of this skill is to turn that movement kit into teachable, testable rooms instead of random platforms.

## Current Project Assumptions

Before giving map advice, anchor to the current code:

- Movement tuning lives in `src/constants.rs`
- Player behavior lives in `src/systems/player.rs`
- Graybox geometry is currently hardcoded in `src/scene.rs` via `spawn_level_geometry`

As of the current project state, the player kit includes:

- ground movement with accel, friction, and turn control
- jump buffer and coyote time
- variable jump height
- wall slide, wall climb, wall jump, and wall kick variants
- dash with freeze, corner correction, and dash end handling
- dash slide and grounded super jump follow-up
- crouch collider changes

Do not propose puzzle rooms that depend on mechanics not yet implemented.

## Collision-First Rule

Design collision before art.

For this project, early level design should use gameplay solids and trigger volumes first, then bind art rules later. Treat visuals as a second pass.

- A map should first define collision boxes, spawn points, room exits, checkpoints, hazards, and optional trigger zones.
- Decorative tiles and sprites should not change the intended route or timing.
- Art hookup can follow a rule like `collision type A -> visual tile set A`, but the room must already play well without that layer.
- When suggesting a layout, describe the collision purpose of each block, not just its appearance.

Preferred collision categories:

- `solid_ground`
- `wall_surface`
- `one_way_platform` if added later
- `hazard`
- `spawn`
- `checkpoint`
- `room_exit`
- `camera_zone`
- `effect_zone`

## Main Rule

Design rooms from verbs, not from shapes.

For every room, first answer:

1. What single player skill is this room teaching or testing?
2. What failure is acceptable here?
3. What recovery space does the player get after a miss?

If a room cannot answer those three questions, simplify it.

## Recommended Room Loop

Build rooms in this order:

1. `Teach`
   Show one mechanic in a low-risk setup.
2. `Reinforce`
   Repeat it with a small twist: tighter timing, reversed side, less runway, or higher commitment.
3. `Combine`
   Pair it with one older mechanic only.
4. `Mastery`
   Ask for a cleaner execution chain.
5. `Release`
   Give a safe platform, checkpoint, or low-pressure traversal beat.

Avoid introducing two new demands in the same room.

## How To Use The Current Movement Kit

### Jump / Coyote / Buffer

Use these to make the game feel fair, not to justify unclear geometry.

- Small gaps are best for teaching pace and confidence.
- Slightly late ledge jumps are now valid, so rooms can feel generous without looking oversized.
- Buffered landing jumps support rhythm sections; place flat landing pads where a quick re-jump is intended.

### Wall Interaction

Use walls to create readable decision points.

- One wall teaches grab or slide.
- Two staggered walls teach transfer and timing.
- A wall next to spikes or a pit raises pressure sharply, so add that only after the base interaction is stable.

### Dash

Dash should create commitment and routing.

- Horizontal dash gaps are the cleanest first lesson.
- Vertical or diagonal dash asks for more recognition and should come later.
- Because dash has corner correction, narrow lip catches can feel good, but do not rely on sub-pixel precision.

### Dash Slide / Super Jump

These are advanced expression mechanics.

- Teach them in a long, readable runway.
- Make the trigger surface visually obvious.
- Give generous ceiling clearance on early attempts.
- Use them as optional speed routes before making them mandatory.

## Graybox Principles

When building early maps in `spawn_level_geometry`, keep geometry readable:

- Prefer large, simple solids over decorative fragmentation.
- Each platform should have one clear purpose: landing, launch, wall setup, dash target, or recovery.
- Leave visible empty space around the intended route so the solution reads from a glance.
- Keep death pits and reset penalties sparse during mechanic discovery.

A good graybox room should still be understandable with all art removed.

When describing graybox, explicitly call out:

- the collision box type
- its gameplay role
- whether it is mandatory route, recovery route, or optional fast route

## First Five Rooms For This Project

If the user asks where to start, suggest this sequence:

1. `Run + jump room`
   Flat runway, one safe gap, one taller landing.
2. `Coyote confidence room`
   A few short ledges that reward moving forward instead of stopping.
3. `Wall room`
   One safe wall climb, then one wall jump transfer.
4. `Dash room`
   A clean horizontal dash gap with an obvious landing block.
5. `Advanced movement room`
   Dash into ground, slide, then super jump into a high landing.

That sequence matches the current codebase better than starting with puzzle-heavy layouts.

## Converting Prototype Geometry Into A Real Map Pipeline

When the user wants to move beyond hardcoded blocks, recommend this order:

1. Extract level geometry from `spawn_level_geometry` into a room data structure.
2. Represent each room as collision and trigger rectangles first.
3. Store each room or map as a standalone JSON file.
4. Add spawn point, room bounds, exits, checkpoints, and camera metadata.
5. Only after room data is stable, consider tilemaps or authored art layers.

For this repository, a practical next step is:

- create `src/level.rs` or `src/level/mod.rs`
- define `MapFile`, `Room`, `CollisionRect`, `SpawnPoint`, `Checkpoint`, and `RoomExit`
- load one JSON room or chapter at startup instead of spawning fixed geometry directly in `scene.rs`

Do not recommend a full tile system before the room logic exists.

## JSON-First Map Structure

Prefer a modular content pipeline:

- one map or chapter can be a JSON file
- one room can also be a JSON file if the project wants finer authoring control
- the runtime should load data and spawn geometry from files, not from hardcoded scene functions

Recommended directory shape:

- `assets/maps/chapter_01.json`
- or `assets/maps/chapter_01/room_00.json`, `room_01.json`

Recommended fields:

- map id
- room id
- room bounds
- spawn point
- collision rectangles
- hazards
- checkpoints
- exits to adjacent rooms
- camera behavior
- optional art tags
- optional music or ambience tags

Example shape:

```json
{
  "id": "chapter_01_room_00",
  "bounds": { "x": 0, "y": 0, "w": 320, "h": 180 },
  "spawn": { "x": 24, "y": 40 },
  "collision": [
    { "kind": "solid_ground", "x": 0, "y": 0, "w": 320, "h": 16, "art_tag": "stone_a" },
    { "kind": "wall_surface", "x": 220, "y": 16, "w": 16, "h": 96, "art_tag": "stone_wall" }
  ],
  "exits": [
    { "side": "right", "target_room": "chapter_01_room_01", "target_spawn": "left_entry" }
  ]
}
```

If the user asks for implementation advice, prefer serde-based JSON loading and keep rendering rules separate from collision data.

## Room-To-Room Continuity

Maps should feel like connected rooms, not isolated test chambers.

When designing a sequence:

- maintain a clear spatial direction such as climbing upward, moving into wind, or crossing a ruined structure
- let the previous room's exit position naturally become the next room's entry position
- keep mechanic progression continuous across adjacent rooms
- use short relief rooms between high-pressure execution rooms

If a room transition is abrupt, add an intermediate bridge room instead of forcing two unrelated challenges together.

## Transition Style Reference

For transitions, use Celeste-like room handoff principles without copying exact content:

- crossing a room boundary should preserve momentum and facing when appropriate
- the player should enter from the corresponding edge of the next room
- camera movement should prioritize clarity over spectacle
- brief transition lock, spawn offset, or camera settle is acceptable if it improves readability
- checkpoints should sit on emotional beats: after mastery, before escalation, or before a long retry chain

Good transition patterns for this project:

- left/right room edge transfer for traversal chains
- upward room continuation after a climb test
- downward recovery room after a failure-heavy challenge

Avoid:

- teleport-like transitions with no spatial relationship
- resetting player state in ways that break learned movement flow unless the design clearly intends it
- changing visual theme or collision language too abruptly between adjacent rooms

## Output Format

When answering the user with this skill, prefer this structure:

1. `Design Goal`
   State the mechanic focus of the room or chapter.
2. `Room Sequence`
   List 3 to 7 rooms in progression order.
3. `Graybox Sketch`
   Describe collision boxes, exits, pits, walls, and recovery zones in plain language.
4. `Implementation Hook`
   Explain whether to edit `spawn_level_geometry` now or introduce JSON room data first.
5. `Risk Check`
   Call out where the design may be too tight, too punishing, or dependent on unimplemented systems.

## Anti-Patterns

Avoid these common mistakes:

- making every challenge lethal
- asking for precision before the room teaches intent
- stacking wall jump, dash, and crouch-tech in the same first-use room
- decorating graybox geometry until readability gets worse
- copying Celeste room silhouettes without matching this project's actual tuning

## Room Template

Use `room_template.md` in this folder when the user wants a fill-in template for planning one room at a time.
