# Feature: Add --crop option to kild-peek screenshot

## Summary

Add `--crop` option to `kild-peek screenshot` command that captures a specific region of a window instead of the full window. After capturing the full window image, crop to the specified pixel coordinates (x, y, width, height) before encoding and saving.

## User Story

As a developer testing specific UI components
I want to capture just a region of a window screenshot
So that I can reduce screenshot file size, focus on specific UI elements, and avoid false diffs from unrelated UI changes

## Problem Statement

When testing specific UI components (header, toolbar, button panel), the current screenshot command captures the entire window. This causes:
1. Larger file sizes for focused tests
2. False positives in diff comparisons when unrelated UI areas change
3. Inability to isolate specific component areas for verification

## Solution Statement

Add a `--crop` flag that accepts pixel coordinates as `x,y,width,height` string. After capturing the full window image as RgbaImage, crop to the specified region using the `image` crate's cropping capabilities, then encode and save the cropped result. Validate crop bounds against captured image dimensions.

## Metadata

| Field            | Value                                             |
| ---------------- | ------------------------------------------------- |
| Type             | ENHANCEMENT                                       |
| Complexity       | LOW                                               |
| Systems Affected | kild-peek CLI, kild-peek-core screenshot module   |
| Dependencies     | image (already in Cargo.toml)                     |
| Estimated Tasks  | 7                                                 |

---

## UX Design

### Before State

```
╔═══════════════════════════════════════════════════════════════════════════════╗
║                              BEFORE STATE                                      ║
╠═══════════════════════════════════════════════════════════════════════════════╣
║                                                                               ║
║   ┌─────────────┐         ┌─────────────┐         ┌─────────────┐            ║
║   │  screenshot │ ──────► │   capture   │ ──────► │  save full  │            ║
║   │   --window  │         │ full window │         │   image     │            ║
║   └─────────────┘         └─────────────┘         └─────────────┘            ║
║                                                                               ║
║   USER_FLOW:                                                                  ║
║   1. kild-peek screenshot --window "KILD" -o full.png                        ║
║   2. Image saved: 1200x800 pixels (entire window)                            ║
║   3. Manual cropping needed if only header area (400x50) is relevant         ║
║                                                                               ║
║   PAIN_POINT: Cannot focus on specific UI components; full window capture    ║
║   includes irrelevant areas that can cause false diff failures               ║
║                                                                               ║
╚═══════════════════════════════════════════════════════════════════════════════╝
```

### After State

```
╔═══════════════════════════════════════════════════════════════════════════════╗
║                               AFTER STATE                                      ║
╠═══════════════════════════════════════════════════════════════════════════════╣
║                                                                               ║
║   ┌─────────────┐         ┌─────────────┐         ┌─────────────┐            ║
║   │  screenshot │ ──────► │   capture   │         │    save     │            ║
║   │   --crop    │         │ full window │         │   cropped   │            ║
║   └─────────────┘         └─────────────┘         └─────────────┘            ║
║                                   │                      ▲                    ║
║                                   ▼                      │                    ║
║                          ┌─────────────┐                 │                    ║
║                          │    CROP     │─────────────────┘                    ║
║                          │  x,y,w,h    │ ◄── extracts region before encode    ║
║                          └─────────────┘                                      ║
║                                                                               ║
║   USER_FLOW:                                                                  ║
║   1. kild-peek screenshot --window "KILD" --crop "0,0,400,50" -o header.png  ║
║   2. Image saved: 400x50 pixels (just the header region)                     ║
║   3. Focused tests, smaller files, no false diffs from other areas           ║
║                                                                               ║
║   VALUE_ADD: Focused component testing without manual post-processing        ║
║                                                                               ║
╚═══════════════════════════════════════════════════════════════════════════════╝
```

### Interaction Changes

| Location       | Before                    | After                                | User Impact                           |
|----------------|---------------------------|--------------------------------------|---------------------------------------|
| `screenshot`   | Full window only          | Optional `--crop "x,y,w,h"` region   | Focused component screenshots         |
| Output size    | Always full window        | Cropped region dimensions            | Smaller files, faster diffs           |
| Error handling | N/A for crop              | Invalid crop bounds error            | Clear feedback on bad coordinates     |

---

## Mandatory Reading

**CRITICAL: Implementation agent MUST read these files before starting any task:**

| Priority | File | Lines | Why Read This |
|----------|------|-------|---------------|
| P0 | `crates/kild-peek-core/src/screenshot/handler.rs` | 351-388 | `encode_image` function - ADD crop logic here before encoding |
| P0 | `crates/kild-peek-core/src/screenshot/types.rs` | 32-107 | `CaptureRequest` builder pattern - ADD `with_crop()` method |
| P0 | `crates/kild-peek/src/app.rs` | 54-111 | CLI screenshot args - ADD `--crop` argument |
| P0 | `crates/kild-peek/src/commands.rs` | 146-228 | `handle_screenshot_command` - PARSE crop arg, pass to request |
| P1 | `crates/kild-peek-core/src/screenshot/errors.rs` | 1-82 | ScreenshotError - ADD crop validation error variant |
| P2 | `crates/kild-peek-core/src/screenshot/types.rs` | 172-255 | Tests - FOLLOW this pattern for new tests |

**External Documentation:**
| Source | Section | Why Needed |
|--------|---------|------------|
| [image crate DynamicImage](https://docs.rs/image/latest/image/enum.DynamicImage.html) | crop_imm() | Cropping method for RgbaImage via DynamicImage |
| [image crate GenericImage](https://docs.rs/image/latest/image/trait.GenericImage.html) | sub_image() | Alternative cropping via sub_image view |

---

## Patterns to Mirror

**CLI_ARGUMENT_PATTERN:**
```rust
// SOURCE: crates/kild-peek/src/app.rs:105-111
// COPY THIS PATTERN for string value argument:
.arg(
    Arg::new("quality")
        .long("quality")
        .help("JPEG quality (1-100, default: 85)")
        .value_parser(clap::value_parser!(u8))
        .default_value("85"),
)
```

**BUILDER_METHOD_PATTERN:**
```rust
// SOURCE: crates/kild-peek-core/src/screenshot/types.rs:100-106
// COPY THIS PATTERN for optional builder method:
pub fn with_jpeg_quality(mut self, quality: u8) -> Self {
    self.format = ImageFormat::Jpeg {
        quality: quality.min(100),
    };
    self
}
```

**ERROR_DEFINITION_PATTERN:**
```rust
// SOURCE: crates/kild-peek-core/src/screenshot/errors.rs:31-32
// COPY THIS PATTERN for new error variant:
#[error("Monitor not found at index: {index}")]
MonitorNotFound { index: usize },
```

**ERROR_CODE_PATTERN:**
```rust
// SOURCE: crates/kild-peek-core/src/screenshot/errors.rs:62
// COPY THIS PATTERN for error code mapping:
ScreenshotError::MonitorNotFound { .. } => "SCREENSHOT_MONITOR_NOT_FOUND",
```

**LOGGING_PATTERN:**
```rust
// SOURCE: crates/kild-peek-core/src/screenshot/handler.rs:376-380
// COPY THIS PATTERN for capture logging:
info!(
    event = "core.screenshot.capture_completed",
    width = width,
    height = height
);
```

**TEST_PATTERN:**
```rust
// SOURCE: crates/kild-peek-core/src/screenshot/types.rs:233-254
// COPY THIS PATTERN for builder/validation tests:
#[test]
fn test_capture_request_jpeg_quality_clamped() {
    let req = CaptureRequest::window("Test").with_jpeg_quality(150);
    match req.format {
        ImageFormat::Jpeg { quality } => assert_eq!(quality, 100),
        _ => panic!("Expected JPEG format"),
    }
}
```

---

## Files to Change

| File                                              | Action | Justification                                    |
| ------------------------------------------------- | ------ | ------------------------------------------------ |
| `crates/kild-peek-core/src/screenshot/types.rs`   | UPDATE | Add `CropArea` struct and `with_crop()` builder  |
| `crates/kild-peek-core/src/screenshot/errors.rs`  | UPDATE | Add `InvalidCropBounds` error variant            |
| `crates/kild-peek-core/src/screenshot/handler.rs` | UPDATE | Add crop logic in `encode_image()` function      |
| `crates/kild-peek/src/app.rs`                     | UPDATE | Add `--crop` CLI argument                        |
| `crates/kild-peek/src/commands.rs`                | UPDATE | Parse crop arg and pass to `CaptureRequest`      |

---

## NOT Building (Scope Limits)

Explicit exclusions to prevent scope creep:

- **Named regions (--crop header)**: Only pixel coordinates; named presets are out of scope
- **Percentage-based cropping**: Only absolute pixel values; percentages would add complexity
- **Interactive crop selection**: CLI tool only; no GUI picker
- **Crop preview/dry-run**: Directly capture and crop; no preview mode
- **Multi-region crops**: Single crop region per invocation; multiple crops not needed
- **Relative positioning (negative values, center-based)**: Only top-left origin coordinates

---

## Step-by-Step Tasks

Execute in order. Each task is atomic and independently verifiable.

### Task 1: ADD `CropArea` type to `types.rs`

- **ACTION**: ADD `CropArea` struct with x, y, width, height fields and add `crop` field to `CaptureRequest`
- **FILE**: `crates/kild-peek-core/src/screenshot/types.rs`
- **IMPLEMENT**:
  ```rust
  /// Region to crop from captured image
  #[derive(Debug, Clone, Copy)]
  pub struct CropArea {
      pub x: u32,
      pub y: u32,
      pub width: u32,
      pub height: u32,
  }

  impl CropArea {
      pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
          Self { x, y, width, height }
      }
  }
  ```
- **ALSO**: Add `pub crop: Option<CropArea>` field to `CaptureRequest` struct
- **ALSO**: Add `with_crop()` builder method mirroring `with_jpeg_quality()` pattern
- **MIRROR**: `crates/kild-peek-core/src/screenshot/types.rs:100-106` for builder pattern
- **VALIDATE**: `cargo build -p kild-peek-core`

### Task 2: ADD `InvalidCropBounds` error variant to `errors.rs`

- **ACTION**: ADD error variant for crop validation failures
- **FILE**: `crates/kild-peek-core/src/screenshot/errors.rs`
- **IMPLEMENT**:
  ```rust
  #[error("Crop region ({x}, {y}, {width}x{height}) exceeds image bounds ({image_width}x{image_height})")]
  InvalidCropBounds {
      x: u32,
      y: u32,
      width: u32,
      height: u32,
      image_width: u32,
      image_height: u32,
  },
  ```
- **ALSO**: Add error code mapping: `ScreenshotError::InvalidCropBounds { .. } => "SCREENSHOT_INVALID_CROP_BOUNDS"`
- **ALSO**: Add to `is_user_error()` match (crop bounds errors are user errors)
- **MIRROR**: `crates/kild-peek-core/src/screenshot/errors.rs:31-32` for error definition
- **VALIDATE**: `cargo build -p kild-peek-core`

### Task 3: ADD crop logic to `encode_image()` in `handler.rs`

- **ACTION**: MODIFY `encode_image()` to accept optional crop area and crop before encoding
- **FILE**: `crates/kild-peek-core/src/screenshot/handler.rs`
- **IMPLEMENT**:
  - Change signature: `fn encode_image(image: image::RgbaImage, format: &ImageFormat, crop: Option<CropArea>) -> Result<CaptureResult, ScreenshotError>`
  - Before encoding, if crop is Some:
    1. Validate crop bounds against image dimensions
    2. Use `image::DynamicImage::ImageRgba8(image).crop_imm(x, y, width, height).to_rgba8()` to crop
    3. Add logging for crop operation
  - Update all callers of `encode_image()` to pass `None` for crop initially
- **MIRROR**: `crates/kild-peek-core/src/screenshot/handler.rs:376-380` for logging
- **GOTCHA**: Use `crop_imm()` not `crop()` - it returns a new image without mutating
- **VALIDATE**: `cargo build -p kild-peek-core && cargo test -p kild-peek-core`

### Task 4: UPDATE capture functions to pass crop from request

- **ACTION**: MODIFY capture dispatch in `capture()` and internal capture functions to pass crop
- **FILE**: `crates/kild-peek-core/src/screenshot/handler.rs`
- **IMPLEMENT**:
  - Update `capture()` to extract crop from request and pass to capture functions
  - Update all `capture_window_*` and `capture_monitor*` functions to accept `crop: Option<CropArea>` and pass to `encode_image()`
- **PATTERN**: Follow existing parameter threading pattern in the file
- **VALIDATE**: `cargo build -p kild-peek-core && cargo test -p kild-peek-core`

### Task 5: ADD `--crop` CLI argument to `app.rs`

- **ACTION**: ADD crop argument to screenshot subcommand
- **FILE**: `crates/kild-peek/src/app.rs`
- **IMPLEMENT**:
  ```rust
  .arg(
      Arg::new("crop")
          .long("crop")
          .help("Crop to region: x,y,width,height (e.g., \"0,0,400,50\")")
          .value_name("REGION"),
  )
  ```
- **LOCATION**: Add after the `quality` argument (around line 111)
- **MIRROR**: `crates/kild-peek/src/app.rs:97-104` for string argument pattern
- **VALIDATE**: `cargo build -p kild-peek`

### Task 6: PARSE crop argument in `commands.rs` and pass to request

- **ACTION**: EXTRACT crop arg, parse as coordinates, add to CaptureRequest
- **FILE**: `crates/kild-peek/src/commands.rs`
- **IMPLEMENT**:
  - Extract: `let crop_str = matches.get_one::<String>("crop");`
  - Parse helper function:
    ```rust
    fn parse_crop_area(s: &str) -> Result<CropArea, Box<dyn std::error::Error>> {
        let parts: Vec<&str> = s.split(',').collect();
        if parts.len() != 4 {
            return Err("Crop format must be x,y,width,height".into());
        }
        let x: u32 = parts[0].trim().parse()?;
        let y: u32 = parts[1].trim().parse()?;
        let width: u32 = parts[2].trim().parse()?;
        let height: u32 = parts[3].trim().parse()?;
        Ok(CropArea::new(x, y, width, height))
    }
    ```
  - Update `build_capture_request()` to accept `crop: Option<CropArea>` parameter
  - Chain `.with_crop(crop)` when building request if crop is Some
  - Add crop to logging event
- **MIRROR**: `crates/kild-peek/src/commands.rs:163-166` for format parsing pattern
- **VALIDATE**: `cargo build -p kild-peek`

### Task 7: ADD tests for crop functionality

- **ACTION**: ADD unit tests for CropArea, parsing, and validation
- **FILE**: `crates/kild-peek-core/src/screenshot/types.rs` (type tests)
- **FILE**: `crates/kild-peek/src/app.rs` (CLI tests)
- **IMPLEMENT**:
  - Test `CropArea::new()` creates correct struct
  - Test `CaptureRequest::with_crop()` builder method
  - Test crop area struct fields are accessible
  - CLI test for `--crop "0,0,100,50"` argument parsing
  - CLI test for combined `--window "X" --crop "0,0,100,50" -o out.png`
- **MIRROR**: `crates/kild-peek-core/src/screenshot/types.rs:233-254` for type tests
- **MIRROR**: `crates/kild-peek/src/app.rs:286-305` for CLI argument tests
- **VALIDATE**: `cargo test -p kild-peek-core && cargo test -p kild-peek`

---

## Testing Strategy

### Unit Tests to Write

| Test File                                        | Test Cases                                     | Validates               |
| ------------------------------------------------ | ---------------------------------------------- | ----------------------- |
| `crates/kild-peek-core/src/screenshot/types.rs`  | CropArea creation, builder method              | Type correctness        |
| `crates/kild-peek-core/src/screenshot/errors.rs` | InvalidCropBounds error display, error_code    | Error handling          |
| `crates/kild-peek/src/app.rs`                    | CLI parsing with/without crop, invalid formats | Argument validation     |

### Edge Cases Checklist

- [ ] Crop area larger than image dimensions (should error)
- [ ] Crop area starting outside image bounds (should error)
- [ ] Crop area at exact image dimensions (should succeed)
- [ ] Crop area with zero width or height (should error or allow?)
- [ ] Invalid crop string format (non-numeric, wrong delimiter)
- [ ] Missing crop values (only 3 numbers)
- [ ] Negative values in crop string (parsed as u32, will fail parse)
- [ ] Whitespace in crop string (trimming should handle)

---

## Validation Commands

### Level 1: STATIC_ANALYSIS

```bash
cargo fmt --check && cargo clippy --all -- -D warnings
```

**EXPECT**: Exit 0, no errors or warnings

### Level 2: UNIT_TESTS

```bash
cargo test -p kild-peek-core && cargo test -p kild-peek
```

**EXPECT**: All tests pass

### Level 3: FULL_SUITE

```bash
cargo test --all && cargo build --all
```

**EXPECT**: All tests pass, build succeeds

### Level 4: MANUAL_VALIDATION

```bash
# Test basic crop functionality
cargo run -p kild-peek -- screenshot --window "Finder" --crop "0,0,100,50" -o /tmp/cropped.png

# Verify output dimensions
file /tmp/cropped.png  # Should show 100x50

# Test invalid crop (should error gracefully)
cargo run -p kild-peek -- screenshot --window "Finder" --crop "10000,0,100,50" -o /tmp/fail.png
# Should output clear error about bounds

# Test crop format validation
cargo run -p kild-peek -- screenshot --window "Finder" --crop "bad" -o /tmp/fail.png
# Should output format error
```

---

## Acceptance Criteria

- [ ] `--crop "x,y,width,height"` flag captures specified region only
- [ ] Invalid crop bounds produce clear error message with actual vs requested dimensions
- [ ] Invalid crop format produces clear parsing error
- [ ] Crop works with all target types (--window, --app, --monitor, --window-id)
- [ ] Crop works with both PNG and JPEG output formats
- [ ] Base64 output includes cropped image when --crop is used
- [ ] Level 1-3 validation commands pass with exit 0
- [ ] Unit tests cover crop area creation, parsing, and validation

---

## Completion Checklist

- [ ] All tasks completed in dependency order
- [ ] Each task validated immediately after completion
- [ ] Level 1: Static analysis (fmt + clippy) passes
- [ ] Level 2: Unit tests pass
- [ ] Level 3: Full test suite + build succeeds
- [ ] Level 4: Manual validation confirms crop works correctly
- [ ] All acceptance criteria met

---

## Risks and Mitigations

| Risk                       | Likelihood | Impact | Mitigation                                                     |
| -------------------------- | ---------- | ------ | -------------------------------------------------------------- |
| image crate API changes    | LOW        | LOW    | Using stable crop_imm() method, well-documented                |
| Performance with large images | LOW     | LOW    | Crop happens in memory before encode, minimal overhead         |
| Coordinate system confusion | MED       | LOW    | Document that (0,0) is top-left, standard image convention     |

---

## Notes

- The `image` crate is already a dependency in kild-peek-core (Cargo.toml line 14)
- Using `crop_imm()` instead of `crop()` because it returns a new image without mutation, matching the functional style of the existing code
- Crop validation happens at encode time, not request creation, because we don't know image dimensions until capture completes
- Error messages include both requested crop region AND actual image dimensions to help users correct their coordinates
