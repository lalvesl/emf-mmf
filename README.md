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
| **Short-pitched**  | on/off  | Chorded coils, to reduce harmonics. Requires `Layers ≥ 2` (see below)      |
| **Layers**         | 1–6     | Conductors packed into each slot (see below)                                |

`S` must be divisible by `2 · p · m`; the panel snaps it to the nearest valid
value and reports the resulting `q = S / (m · P)`, slot angle and phase angle.

Besides the winding parameters, the panel toggles the endwinding arcs, the MMF
arrows, the MMF field overlay (per phase and resultant), the rotor, and the
winding-scheme window.

**Layers** is how many conductors sit in each slot. Two or more split the slot
into two electrical layers, which is what makes short-pitching possible — with
a single layer the option is unavailable.

## What you can see

- **The stator in 3D** — slots, conductors coloured by phase, and the coil
  heads arcing between them.
- **The MMF field** — per phase and resultant, animating with the currents.
  A lobe's colour is its phase (white for the resultant), its outer rim is the
  magnetic polarity (red north, blue south), and its opacity is the magnitude.
- **MMF arrows** — one per phase per pole, plus the resultant, which turns at
  synchronous speed alongside the rotor.
- **The winding diagram** — slot-by-slot conductor layout and the MMF waveform,
  in a separate window.
- **The current waveforms** — playable and scrubbable, driving everything else.

With the optional [`harmonics`](#cargo-features) feature the panel also reports
the winding factors `k_d`, `k_p` and `k_w`, and the winding diagram gains a
harmonic spectrum showing what short-pitching does to the 5th and 7th.

## Cargo features

| Feature | Default | What it adds |
| :------------ | :-----: | :----------------------------------------------------------------- |
| `harmonics`   | off     | Winding factors (`k_d`, `k_p`, `k_w`) and the harmonic spectrum panel |
| `web`         | off     | WebAssembly target adjustments                                       |
| `dev`         | off     | `bevy/dynamic_linking`, for faster incremental links                 |
| `hotpatching` | off     | `bevy/hotpatching`, so `dx serve` can re-register systems live       |

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
