# Experiment Sequencer

Visual timeline-based experiment builder for rust-daq GUI.

## Features

### Action Palette
- **Move Absolute** (→) - Move stage to absolute position
- **Move Relative** (↔) - Move stage by relative amount
- **Set Parameter** (⚙) - Set device parameter (exposure, wavelength, power)
- **Trigger** (📷) - Trigger camera or acquisition
- **Read** (📊) - Read scalar value from device
- **Loop** (🔄) - Repeat actions N times (can contain nested actions)
- **Delay** (⏱) - Wait for specified duration
- **Parallel** (⫴) - Execute actions in parallel (container for nested actions)

### Drag-and-Drop Timeline
- Drag actions from palette to timeline
- Reorder actions by dragging
- Nest actions inside Loop and Parallel blocks
- Delete actions with trash icon
- Select actions to edit parameters

### Parameter Inspector
- Right panel shows parameters for selected action
- Device selector populated from connected devices
- Number inputs with units (mm, seconds, nm, etc.)
- Boolean toggles for wait flags
- Validation for required fields

### Real-time Script Preview
- Bottom panel shows generated code
- Switch between Rhai and Python output
- Updates in real-time as you build sequence
- Syntax highlighting with Monaco Editor

### Save/Load Functionality
- Save experiment plans as JSON files
- Load saved plans
- Export to Rhai (.rhai) or Python (.py) scripts
- "Edit Code" button for switching to code mode (placeholder)

## Usage

1. **Connect to daemon** - Click connection button in header
2. **Switch to Sequencer** - Click "Sequencer" tab in header
3. **Build sequence** - Drag actions from palette to timeline
4. **Configure parameters** - Click action blocks to edit in right panel
5. **Preview script** - See generated code in bottom panel
6. **Save plan** - Click "Save" button to save as JSON
7. **Export script** - Click "Export" in script preview to save as .rhai or .py

## Example Workflow

### Simple Stage Scan
```
1. Move Absolute (stage_x, 0.0 mm, wait=true)
2. Loop (10 iterations, variable=i)
   - Trigger (camera, 1 frame)
   - Move Relative (stage_x, 1.0 mm, wait=true)
3. Delay (1.0 seconds)
```

Generated Rhai:
```rhai
move_absolute("stage_x", 0.0, true);
for i in 0..10 {
    trigger("camera");
    move_relative("stage_x", 1.0, true);
}
sleep(1.0);
```

### Wavelength Sweep with Power Measurement
```
1. Loop (20 iterations, variable=i)
   - Set Parameter (laser, wavelength, 700 + i*5)
   - Delay (0.5 seconds)
   - Read (power_meter, variable=power)
   - Trigger (camera, 1 frame)
```

Generated Python:
```python
for i in range(20):
    client.set_wavelength("laser", 700 + i*5)
    time.sleep(0.5)
    power = client.read("power_meter")
    client.trigger("camera")
```

## File Format

Experiment plans are saved as JSON:

```json
{
  "name": "My Experiment",
  "description": "",
  "created": "2025-12-03T...",
  "modified": "2025-12-03T...",
  "actions": [
    {
      "id": "action-1234567890-abc123",
      "type": "move_absolute",
      "params": {
        "device": "stage_x",
        "position": 0.0,
        "wait": true
      }
    },
    {
      "id": "action-1234567891-def456",
      "type": "loop",
      "params": {
        "iterations": 10,
        "variable": "i"
      },
      "children": [...]
    }
  ]
}
```

## Architecture

### Components
- `ExperimentSequencer.tsx` - Main container with DragDropContext
- `ActionPalette.tsx` - Left panel with draggable action templates
- `Timeline.tsx` - Center panel with drop zone for actions
- `ActionBlockComponent.tsx` - Individual action block (recursive for nesting)
- `ParameterInspector.tsx` - Right panel for editing parameters
- `ScriptPreview.tsx` - Bottom panel with Monaco Editor

### Types
- `types/experiment.ts` - Type definitions for actions, templates, and plans
- Action types: move_absolute, move_relative, set_parameter, trigger, read, loop, delay, parallel
- Parameter types: string, number, boolean, device, select

### Script Generation
- `utils/scriptGenerator.ts` - Converts action blocks to code
- `generateRhaiScript()` - Generates Rhai scripting language
- `generatePythonScript()` - Generates Python with rust-daq client

## Limitations

- **Run button disabled** - Experiment execution requires gRPC methods not yet implemented
- **Parallel execution** - Rhai doesn't support true parallelism (runs sequentially)
- **Device validation** - Parameters not validated against device capabilities
- **Loop expressions** - Cannot use expressions in parameters (only literals)
- **Error handling** - No try/catch blocks in generated scripts

## Future Enhancements

See bd-rbqk (Phase 2 parent issue) for planned improvements:

- Execute experiments directly from GUI (requires gRPC integration)
- Live data visualization during experiment execution
- Parameter sweeps with calculated expressions
- Conditional branching (if/else blocks)
- Save experiment results with plan metadata
- Template library for common experiment patterns
- Undo/redo functionality
- Copy/paste action blocks
- Keyboard shortcuts

## Dependencies

- `react-beautiful-dnd` - Drag-and-drop functionality
- `@monaco-editor/react` - Code editor for script preview
- `@tauri-apps/api` - File system operations (save/load)
- `lucide-react` - Icons for UI elements
