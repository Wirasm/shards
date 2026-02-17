# Focus Management

## FocusHandle

```rust
struct MyView {
    focus_handle: FocusHandle,
}

impl MyView {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }
}
```

## Focusable Trait

Required for views that receive keyboard events:

```rust
impl Focusable for MyView {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
```

## Rendering with Focus

```rust
impl Render for MyView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_focused = self.focus_handle.is_focused(window);

        div()
            .track_focus(&self.focus_handle) // Enable focus tracking
            .on_key_down(cx.listener(Self::on_key_down))
            .when(is_focused, |el| {
                el.border_color(theme::ice()) // Visual focus indicator
            })
            .child("Focusable content")
    }
}
```

## Focus Operations

```rust
// Give focus to this element
window.focus(&self.focus_handle);

// Check focus state
self.focus_handle.is_focused(window)    // This exact element
self.focus_handle.contains_focused(window) // This or any descendant

// Remove focus
cx.blur();
```

## Focus Events

```rust
div()
    .track_focus(&self.focus_handle)
    .on_focus(cx.listener(|this, _event, cx| {
        this.on_focus(cx);
    }))
    .on_blur(cx.listener(|this, _event, cx| {
        this.on_blur(cx);
    }))
```

## Propagating Keys

When a focused element doesn't handle a key, propagate to parent:

```rust
fn on_key_down(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
    match event.keystroke.key.as_str() {
        "enter" => self.submit(cx),
        _ => cx.propagate(), // Let parent handle it
    }
}
```

## Tab Navigation

Elements with `.track_focus()` automatically participate in Tab/Shift-Tab navigation.

## Auto-Focus on Mount

```rust
impl MyDialog {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        window.focus(&focus_handle); // Focus immediately

        Self { focus_handle }
    }
}
```
