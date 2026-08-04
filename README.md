# EMF-MMF — Electrical Motor Coil Winding Simulator

**EMF-MMF** is a 3D interactive simulation for visualizing and configuring the coil winding schemas of electrical motors. The application renders a **3D cylindrical stator** with grooves (slots), through which conductive wires pass. Wires are connected to form polyphase electrical motor windings, allowing the user to experiment with different motor configurations in real time.

The project is inspired by classical stator winding diagrams — a circular cross-section of a stator showing numbered grooves with colored coil bundles routed through them, and the corresponding waveform representation of the winding pattern.

## Core Concept

```
     ┌─┐   ┌─┐   ┌─┐
     │ │   │ │   │ │        ← Winding waveform (coil pitch)
  ───┘ └───┘ └───┘ └───

     ╭──── Stator ────╮
    ╱  ┌──┐      ┌──┐  ╲
   │  N_a │      │N_b │  │  ← Grooves with wire bundles
   │  └──┘      └──┘  │     (polyphase coloring)
    ╱  ┌──┐      ┌──┐  ╱
     ╰─┤N_c│────│N_d├─╯
        └──┘    └──┘
```

The simulation models:

1.  **Stator grooves (slots)** — The physical slots machined into the stator core, arranged in a cylindrical pattern.
2.  **Wire routing** — Conductive wires threaded through the grooves, forming coil groups.
3.  **Polyphase connections** — The wires are interconnected to create a polyphase (e.g., 3-phase) winding configuration.
4.  **3D visualization** — A rendered 3D cylinder representing the stator, with wires visible inside the grooves and their connections.

## Configurable Parameters (UI)

The application provides a UI panel to configure the following motor winding parameters:

| Parameter          | Range   | Description                                                                |
| ------------------ | ------- | -------------------------------------------------------------------------- |
| **Grooves (S)**    | 6–144   | Total number of slots/grooves in the stator                                |
| **Phases (m)**     | 2–10    | Number of electrical phases                                                |
| **Poles (P)**      | 2–12    | Total magnetic poles, always even (pole pairs `p` = P/2)                    |
| **Short-pitched**  | on/off  | Whether the winding uses short-pitched (chorded) coils to reduce harmonics |
| **Layers**         | 1–6     | Conductors packed into each slot (see below)                                |

`S` must be divisible by `2 · p · m`; the panel snaps it to the nearest valid
value and reports the resulting `q = S / (m · P)`, slot angle and phase angle.

Besides the winding parameters, the panel toggles the endwinding arcs, the MMF
arrows, the MMF field overlay (per phase and resultant), the rotor, and the
winding-scheme window.

### Slot filling and electrical layers

**Layers** is the number of round conductors packed into a slot. They are laid
out two per row, filling from the slot bottom towards the bore — `4` gives a
2×2 stack, `6` gives 2×3:

```
        slot bottom (r = bore + SLOT_DEPTH)
        ┌───────────────┐
        │  (A)     (A)  │  deep half  → starts the coil at this slot
        │  (C')    (C') │  shallow half → returns the coil from slot i-pitch
        └───────────────┘
        bore  (r = STATOR_BORE_RADIUS)
```

The stack is split into two *electrical* layers. The deep half carries the
outgoing side of the coil starting at this slot; the shallow half carries the
return side of the coil that started `coil_pitch` slots earlier — same phase,
reversed direction.

- **Full pitch** → both halves resolve to the same phase, so the slot behaves
  exactly like a single-layer winding.
- **Short-pitched** → the halves disagree at the belt boundaries, putting two
  different phases in one slot. That is what chording physically is, and it is
  only possible because the slot has two layers.

`Layers = 1` collapses to the plain single-layer winding. Odd counts give the
extra conductor to the deep half.

Endwinding arcs are drawn one per conductor, at the same gauge as the wire, and
sweep radially so each arc actually meets the shallow conductor it connects to.

With the endwindings shown, the slot conductors extend to sit flush with the
core face and each arc begins with a straight axial run down to that same point,
coaxial with the wire — the two butt together into a single continuous
conductor. The straight run also lifts the arc clear of the face before it
starts sweeping, so it does not cut through the teeth:

```
          ╭──────────────╮   ← arc (lift staggered by phase and conductor)
          │              │
          ╵              ╵   ← straight lead, clears the core face
   ═══════╪══════════════╪═══  core face (y = STATOR_HEIGHT/2)
          ║              ║   ← slot conductor, same axis and gauge
```

With the endwindings hidden the conductors shrink back inside the core so the
current-direction symbols on their end faces stay readable.

## Tech Stack

- **Rust**: Core language for performance and safety (edition 2024).
- **Bevy 0.19**: ECS-based 3D game engine for rendering.
- **bevy_egui 0.41**: For the configuration panels and 2D diagrams.
- **Nix Flakes**: Reproducible development and build environments.
- **Dioxus CLI**: Used as a development tool for hot-patching and multi-platform builds.

## Getting Started

### Prerequisites

- [Nix](https://nixos.org/) with flakes enabled.
- Directly using `cargo` is possible, but Nix is recommended for a reproducible environment.

### Development Environment

Enter the development shell to access all tools:

```bash
nix develop
```

The shell uses **Rust Nightly** with the **Cranelift** codegen backend for fast incremental compilation.

### Run (via Nix apps)

The `apps` in `flake.nix` drive the Dioxus CLI (`dx`) for local development:

| Action            | Command                   | Description                                                          |
| :---------------- | :------------------------ | :------------------------------------------------------------------- |
| **Run Dev**       | `nix run`                 | Runs `dx serve` with hot-patching for local development (Native).    |
| **Run Web**       | `nix run .#web`           | Runs `dx serve` with the web platform target.                        |
| **Build Web**     | `nix run .#build-web`     | `dx build` for a WebAssembly bundle.                                 |
| **Build Linux**   | `nix run .#build-linux`   | `dx build --release` for Linux.                                      |
| **Build Windows** | `nix run .#build-windows` | Cross-compiles a release binary for Windows (x86_64-pc-windows-gnu). |

### Build (via Nix packages)

The `packages` are the reproducible, sandboxed builds — these are what CI runs.
They call `cargo` directly and do not need the Dioxus CLI:

| Command             | Output                                                     |
| :------------------ | :--------------------------------------------------------- |
| `nix build`         | Linux binary at `result/bin/emf-mmf` (alias of `.#linux`). |
| `nix build .#web`   | WASM bundle + `index.html` in `result/`.                   |
| `nix build .#windows` | `result/bin/emf-mmf.exe`.                                |
| `nix build .#android` | `result/emf-mmf.apk`.                                    |

## Project Structure

- `src/main.rs` / `src/lib.rs`: Entry point and plugin registration.
- `src/config.rs`: `MotorConfig` resource, geometry constants, and the
  `MotorConfigChanged` message that drives every regeneration.
- `src/stator.rs`: Procedural stator mesh generation (yoke + teeth).
- `src/winding/`: Phase-belt distribution, slot conductors, endwinding arcs and
  current-direction symbols.
- `src/electrical.rs`: Electrical angle animation and the current waveform strip.
- `src/mmf_field/`: MMF field overlay meshes (per phase and resultant).
- `src/vectors/`: 3D MMF arrows per phase and pole.
- `src/rotor/`: Rotor geometry, synchronised to the resultant MMF.
- `src/winding_scheme/`: 2D winding diagram and MMF waveform window.
- `src/phase/`: Phase colours and letters.
- `src/ui.rs`: Main configuration panel (`bevy_egui`).
- `src/i18n.rs`: PT-BR / EN string table.
- `src/camera.rs`: Orbit camera controller for 3D exploration.
- `flake.nix`: Nix configuration for development and build automation.

---

## License

This project is open-source. Please check the `LICENSE` file for more details.
