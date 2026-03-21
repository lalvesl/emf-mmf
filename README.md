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

| Parameter          | Description                                                                |
| ------------------ | -------------------------------------------------------------------------- |
| **Grooves Number** | Total number of slots/grooves in the stator (12–72)                        |
| **Phases**         | Number of electrical phases (e.g., 1, 3, 6…)                               |
| **Short-pitched**  | Whether the winding uses short-pitched (chorded) coils to reduce harmonics |
| **Layers**         | Number of winding layers per groove (single-layer or double-layer)         |
| **Pole Pairs**     | Number of magnetic pole pairs (1–4)                                        |

## Tech Stack

- **Rust**: Core language for performance and safety.
- **Bevy 0.18**: ECS-based 3D game engine for rendering.
- **bevy_egui**: For the configuration side-panel.
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

### Run and Build (via Nix Flakes)

The project includes several `apps` defined in `flake.nix` for easy building and execution:

| Action            | Command                   | Description                                                          |
| :---------------- | :------------------------ | :------------------------------------------------------------------- |
| **Run Dev**       | `nix run`                 | Runs `dx serve` with hot-patching for local development (Native).    |
| **Run Web**       | `nix run .#web`           | Runs `dx serve` with the web platform target.                        |
| **Build Web**     | `nix run .#build-web`     | Compiles a production-ready WebAssembly bundle.                      |
| **Build Linux**   | `nix run .#build-linux`   | Compiles a release binary for Linux.                                 |
| **Build Windows** | `nix run .#build-windows` | Cross-compiles a release binary for Windows (x86_64-pc-windows-gnu). |

## Project Structure

- `src/main.rs`: Application entry point and plugin registration.
- `src/stator.rs`: Procedural stator mesh generation logic.
- `src/winding.rs`: Algorithms for winding distribution and wire mesh generation.
- `src/ui.rs`: UI panel for real-time configuration using `bevy_inspector_egui`.
- `src/camera.rs`: Orbit camera controller for 3D exploration.
- `flake.nix`: Nix configuration for development and build automation.

---

## License

This project is open-source. Please check the `LICENSE` file for more details.
