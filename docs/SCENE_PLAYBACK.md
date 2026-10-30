# Scene Playback Engine

Atelier provides a standalone hardware-accelerated 2D scene playback engine capable of importing and executing Wallpaper Engine `scene.pkg` projects without requiring Steam or Wallpaper Engine to be running.

```mermaid
flowchart TD
    subgraph Import ["Import & Decompression (crates/wallpaper & crates/ui)"]
        PKG["scene.pkg Archive"] --> PkgReader["package.rs (Archive Extractor)"]
        PkgReader --> TEX["TEXB0001-0004 Textures"]
        PkgReader --> MDL["MDLV0023 Puppet Meshes"]
        PkgReader --> Manifest["scene.json / project.json"]

        TEX --> TexDecoder["texture.rs (LZ4, DXT1/3/5, RGBA)"]
        MDL --> MeshResolver["scene_geometry.rs (Bind Poses, Bones)"]
        Manifest --> PropEvaluator["scene_properties.rs (Author Bindings)"]
    end

    subgraph Runtime ["WebGL 2D Playback Pipeline (crates/ui/ui/scene-player.js)"]
        LayerTree["Hierarchical Layer Graph<br/>(Parent-Child Transforms, Opacity)"]
        Shaders["Custom GLSL Shaders<br/>(Water Ripple, Flow, Shake, Pulse)"]
        Compositor["Post-Process Compositing Passes<br/>(Masked Blur, Bloom, God Rays, FXAA)"]
        ColorGrading["Final Color Grading<br/>(Hue, Saturation, Brightness, Contrast, Tint)"]
    end

    TexDecoder --> LayerTree
    MeshResolver --> LayerTree
    PropEvaluator --> LayerTree

    LayerTree --> Shaders
    Shaders --> Compositor
    Compositor --> ColorGrading
    ColorGrading --> DesktopCanvas["Desktop Canvas (WorkerW Layer)"]
```

---

## Architectural Highlights

### 1. Layer Hierarchy and Mesh Deformation
- **Hierarchical Transforms**: Propagates 2D transformation matrices (position, rotation, scale, visibility) from parent containers down to child layers.
- **Puppet Meshes (`MDLV0023`)**: Parses static 2D puppet mesh vertex coordinates, UV maps, and attachment bind poses without running heavy external skeletal runtime libraries.
- **Composite Passes**: Evaluates post-processing effects in strict author source order, rendering into ping-pong offscreen framebuffers.

```mermaid
sequenceDiagram
    autonumber
    participant UI as Studio Settings UI
    participant Parser as scene_properties.rs
    participant GL as scene-player.js (WebGL)
    participant Canvas as Desktop Canvas

    UI->>Parser: Update Author Property (e.g. Character Variant = "Asuna")
    Parser->>Parser: Evaluate declarative display conditions (no eval)
    Parser->>GL: update_bindings({ layer_id: "variant_asuna", visible: true })
    GL->>GL: Toggle layer visibility & bind pose
    GL->>GL: Re-render layer graph with new uniform bindings
    GL-->>Canvas: Composite frame rendered
```

---

## Feature Compatibility Matrix

| Category | Supported Features | Unsupported / Limitations |
| :--- | :--- | :--- |
| **Package & Container** | `PKGV0001`, `PKGV0002`, direct folder imports | Password-protected or encrypted packages |
| **Textures & Formats** | `TEXB0001`–`TEXB0004`, LZ4, DXT1, DXT3, DXT5, R8, RG88, PNG, JPEG | Animated texture atlases, animated GIFs |
| **Geometry & Meshes** | 2D image sprites, `MDLV0023` puppet meshes, attachment points, bind poses | 3D models (`.obj`, `.mdl3`), dynamic skeletal bone animation |
| **Shaders & Effects** | Water waves, ripples, flow maps, camera shake, pulse, masked blur, bloom, god rays, built-in rain | Arbitrary author custom HLSL/GLSL shaders, complex 3D particle systems |
| **Audio** | Looping MP3, OGG, WAV audio playback, volume and mute controls | Author SceneScript audio events |
| **Properties & Logic** | Boolean switches, sliders, color pickers, combos, declarative visibility conditions | Arbitrary JavaScript `SceneScript`, file selectors, external application shortcuts |

---

## Author Property Bindings & Security
- **No `eval` Execution**: Author visibility expressions are evaluated using a bounded arithmetic and logical AST parser (`scene_properties.rs`). Arbitrary JavaScript execution is impossible.
- **Deterministic State Swapping**: When importing or upgrading scenes, Atelier stages the entire target structure before swapping directories atomically, preventing corrupted or half-extracted assets from locking the renderer.

---

## Debugging and Diagnostic CLI Flags

```powershell
# Run headless GPU verification test bench
Atelier.exe --scene-engine-test path/to/scene.pkg output-dir/report.json

# Check static cover compositor output
Atelier.exe --scene-preview-test path/to/scene.pkg output.png

# Test author property controls and settings UI
Atelier.exe --scene-settings-test path/to/scene.pkg report.json
```
