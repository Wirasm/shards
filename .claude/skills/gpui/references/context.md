# Context Types

## Context Hierarchy

```
App (Global)
  └─ Window (Per-window)
       └─ Context<T> (Per-component)
            └─ AsyncApp (In async tasks)
```

## App — Global Context

Available at app startup and in global operations.

```rust
Application::new().run(|cx: &mut App| {
    // Create entities
    let entity = cx.new(|cx| MyState::default());

    // Set globals
    cx.set_global(AppSettings { ... });

    // Open windows
    cx.open_window(WindowOptions::default(), |window, cx| {
        cx.new(|cx| Root::new(view, window, cx))
    });

    // Bind keys
    cx.bind_keys([...]);
});
```

## Window — Window Context

Window-specific operations, available in `Render::render()`.

```rust
impl Render for MyView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_focused = window.is_window_focused();
        let bounds = window.bounds();

        // Focus management
        window.focus(&self.focus_handle);

        // Text measurement
        let line = window.text_system().shape_line(...);
        let line_height = window.line_height();

        div().child("Content")
    }
}
```

## Context<T> — Component Context

Available inside `Render::render()` and entity update closures.

```rust
impl MyView {
    fn do_stuff(&mut self, cx: &mut Context<Self>) {
        cx.notify();                    // Trigger re-render
        cx.emit(MyEvent::Changed);      // Emit typed event
        cx.spawn(async move |...| {});  // Spawn async task
        cx.focus_handle();              // Create focus handle
        let entity = cx.entity();       // Get self as Entity<Self>
        let weak = entity.downgrade();  // Weak reference

        // Create child entities
        let child = cx.new(|cx| ChildState::default());

        // Observe/subscribe
        cx.observe(&entity, |this, observed, cx| { ... }).detach();
        cx.subscribe(&entity, |this, emitter, event, cx| { ... }).detach();

        // Background executor
        let executor = cx.background_executor().clone();
    }
}
```

## AsyncApp — Async Context

Available inside `cx.spawn()` closures. Limited to entity operations.

```rust
cx.spawn(async move |this, cx: &mut gpui::AsyncApp| {
    // `this` is WeakEntity<Self>
    let _ = this.update(cx, |view, cx| {
        view.data = data;
        cx.notify();
    });

    // Access background executor
    cx.background_executor().timer(Duration::from_secs(1)).await;
}).detach();
```

## Key Rules

| Operation | App | Window | Context<T> | AsyncApp |
|-----------|-----|--------|------------|----------|
| Create entities | Yes | No | Yes | No |
| Spawn tasks | No | No | Yes | No |
| Notify/emit | No | No | Yes | No |
| Update entities | Yes | No | Yes | Yes |
| Read entities | Yes | No | Yes | Yes |
| Open windows | Yes | No | No | No |
| Focus management | No | Yes | No | No |
| Text measurement | No | Yes | No | No |
| Bind keys | Yes | No | No | No |
