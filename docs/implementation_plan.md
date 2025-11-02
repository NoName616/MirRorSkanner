## MirorSkaner Remediation Plan

### Objectives
- Bring the Rust/Iced port in line with the 27.08.2025 technical specification.
- Ensure the application builds and runs (degraded but functional) on Windows and Linux.
- Provide asynchronous, modular architecture integrating controller, camera, scanning engine, and UI.

### Work Breakdown Structure

1. **FFI & Platform Abstraction**
   - Introduce a `camera::backend` module exposing a `CameraBackend` trait.
   - Provide a Windows implementation that dynamically loads Connect SDK symbols and surfaces rich errors.
   - Supply a cross-platform mock backend (configurable via CLI/env) to satisfy Linux builds and enable dry runs.
   - Gate DLL linking behind `cfg(target_os = "windows")`; replace direct `#[link]` usage with `libloading`.

2. **Serial Controller Service**
   - Convert blocking serial thread to Tokio tasks with structured command/response handling.
   - Load baud rate, timeouts, retry budgets from `config.cfg`.
   - Surface controller status via async channels and notify UI.
   - Implement reconnect logic and error backoff.

3. **Scanning Engine**
   - Create `processing::scan_engine` module orchestrating trajectory execution.
   - Sequence: home (optional), move, dwell, capture Tmin/Tmax, deduplicate, persist.
   - Support fast and precise analysis pipelines selectable per point.
   - Persist results through `data::storage` + `data::export`.

4. **Camera Pipeline**
   - Build async frame acquisition loop delivering frames to subscribers.
   - Implement fast (downsample/ROI) and precise (median filter) analyzers.
   - Maintain thermal texture for UI visualization and calibration workflows.

5. **Configuration & Logging**
   - Refactor `ConfigManager` into a shared service with change notifications.
   - Add subscriptions for hot reload, propagate updates to dependent systems.
   - Integrate structured logging macros with context (component, correlation ids).

6. **UI/UX Overhaul**
   - Reorganize UI into `components/`, `panels/`, `layouts/` maintaining modular structure.
   - Implement dark glassmorphism theme, adaptive layout, live preview, diagnostics, and debug console.
   - Introduce command palette for manual COM commands and camera control toggles.

7. **Testing Strategy**
   - Unit tests: angle parsing, trajectory generation, unit conversions, deduplication.
   - Integration tests: mock controller/camera verifying scan execution.
   - Snapshot tests for configuration serialization/deserialization.

8. **Documentation**
   - Update `CHANGELOG.md` and author execution notes.
   - Provide developer onboarding guide covering hardware requirements and mock modes.

### Sequencing & Dependencies
1. Establish backend abstractions (FFI/mocks) to unblock cross-platform builds.
2. Implement serial service + scanning engine skeleton to drive data flow.
3. Integrate camera pipeline and analysis.
4. Refactor UI and connect to services.
5. Finalize testing + documentation.

### Risk Mitigation
- **SDK unavailability**: default to mock backend with warning; allow runtime selection.
- **Timing-sensitive I/O**: use bounded channels, configurable timeouts, extensive logging.
- **Large refactor**: incrementally land changes with module-level tests.

### Deliverables
- Updated source tree complying with technical specification.
- Automated tests covering core algorithms and service orchestration.
- Revised documentation and changelog entries summarizing remediation.
