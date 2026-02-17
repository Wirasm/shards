# Custom Elements (Low-Level Element Trait)

## When to Use

Use `impl Element` when you need fine-grained control over layout, prepaint, and paint phases. **Prefer `Render`/`RenderOnce` for standard components.**

Use cases:
- Custom layout algorithms
- Performance-critical rendering (terminal, canvas)
- Direct GPU draw calls (text shaping, rectangles)

## Three-Phase Rendering

```
request_layout → RequestLayoutState →
prepaint → PrepaintState →
paint
```

## Minimal Implementation

```rust
use gpui::{
    App, Bounds, Element, ElementId, GlobalElementId, Hitbox, HitboxBehavior,
    InspectorElementId, IntoElement, LayoutId, Pixels, Size, Style, Window,
    px, size,
};

pub struct MyElement {
    // your fields
}

impl IntoElement for MyElement {
    type Element = Self;
    fn into_element(self) -> Self::Element { self }
}

impl Element for MyElement {
    type RequestLayoutState = ();       // Data from layout → prepaint/paint
    type PrepaintState = Hitbox;        // Data from prepaint → paint

    fn id(&self) -> Option<ElementId> { None }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> { None }

    // Phase 1: Calculate size
    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let style = Style {
            size: Size {
                width: gpui::relative(1.).into(),
                height: gpui::relative(1.).into(),
            },
            ..Default::default()
        };
        (window.request_layout(style, [], cx), ())
    }

    // Phase 2: Create hitboxes, prepare data
    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
        window.insert_hitbox(bounds, HitboxBehavior::Normal)
    }

    // Phase 3: Render and handle interactions
    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _layout: &mut Self::RequestLayoutState,
        hitbox: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        // Paint background
        window.paint_quad(gpui::fill(bounds, cx.theme().background));

        // Handle mouse events
        window.on_mouse_event({
            let hitbox = hitbox.clone();
            move |event: &gpui::MouseDownEvent, phase, window, cx| {
                if hitbox.is_hovered(window) && phase.bubble() {
                    // Handle click
                    cx.stop_propagation();
                }
            }
        });

        // Set cursor style
        window.set_cursor_style(gpui::CursorStyle::PointingHand, hitbox);
    }
}
```

## Key Concepts

### State Flow

State flows forward through associated types:
- `RequestLayoutState`: computed in layout, consumed in prepaint + paint
- `PrepaintState`: computed in prepaint, consumed in paint

Use `()` when no state is needed. Use structs for complex state:

```rust
struct MyLayoutState {
    cell_width: Pixels,
    cell_height: Pixels,
}

struct MyPaintState {
    hitbox: Hitbox,
    visible_range: Range<usize>,
}
```

### Hitboxes

```rust
// Create in prepaint (NEVER in paint)
let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);

// Normal: blocks events from passing through
// Transparent: allows events through to elements below

// Use in paint
if hitbox.is_hovered(window) { /* ... */ }
window.set_cursor_style(CursorStyle::IBeam, &hitbox);
```

### Mouse Events

```rust
window.on_mouse_event({
    let hitbox = hitbox.clone();
    move |event: &MouseDownEvent, phase, window, cx| {
        if !phase.bubble() { return; }
        if !hitbox.is_hovered(window) { return; }
        // Handle event
        cx.stop_propagation();
    }
});
```

Event types: `MouseDownEvent`, `MouseUpEvent`, `MouseMoveEvent`, `ScrollWheelEvent`

### Performance

- **No allocations in paint** — pre-compute in `request_layout` or `prepaint`
- **Only paint visible children** in scrollable containers
- **Cache expensive computations** between frames
- Hitboxes in prepaint, events in paint — never reversed
