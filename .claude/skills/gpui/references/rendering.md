# Rendering: Render and RenderOnce

## Render Trait (Stateful Views)

Used for components with mutable state that persist across frames.

```rust
use gpui::{Context, IntoElement, Render, Window, div, prelude::*};

pub struct MyView {
    name: String,
    count: usize,
}

impl Render for MyView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_focused = self.focus_handle.is_focused(window);

        div()
            .flex()
            .flex_col()
            .child(format!("Hello, {}!", self.name))
            .when(self.count > 0, |el| {
                el.child(format!("Count: {}", self.count))
            })
    }
}
```

**Key points:**
- `&mut self` — can read and mutate state
- `window: &mut Window` — access window state (focus, bounds, text system)
- `cx: &mut Context<Self>` — spawn tasks, notify, emit events
- Called every frame when `cx.notify()` triggers re-render

### Creating Views

```rust
// In an App context (e.g., main.rs)
let view = cx.new(|cx| MyView { name: "World".into(), count: 0 });

// In another component
let child = cx.new(|cx| ChildView::new(cx));
```

## RenderOnce Trait (Stateless Components)

Used for one-shot components consumed during rendering. Cannot be re-rendered.

```rust
use gpui::{IntoElement, RenderOnce, Window, div, prelude::*};

#[derive(IntoElement)]
pub struct Badge {
    label: String,
    color: Hsla,
}

impl RenderOnce for Badge {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .px(px(8.))
            .py(px(2.))
            .rounded(px(4.))
            .bg(self.color)
            .child(self.label)
    }
}

// Usage: div().child(Badge { label: "New".into(), color: green() })
```

**Key differences from Render:**
- Takes `self` (consumed), not `&mut self`
- `#[derive(IntoElement)]` required
- No `Context<Self>` — gets `&mut App` instead
- Cannot call `cx.notify()`, spawn tasks, or emit events

## When to Use Which

| Need | Use |
|------|-----|
| Persistent state, re-renders | `impl Render` |
| Stateless display component | `impl RenderOnce` |
| Custom layout/paint control | `impl Element` (see [elements.md](elements.md)) |

## Conditional Rendering

```rust
div()
    // Conditional child
    .when(condition, |el| el.child("Shown when true"))

    // Conditional with value
    .when_some(optional_value, |el, value| {
        el.child(format!("Value: {}", value))
    })

    // Multiple children from iterator
    .children(items.iter().map(|item| {
        div().child(item.name.clone())
    }))
```
