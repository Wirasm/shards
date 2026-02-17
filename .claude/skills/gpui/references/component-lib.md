# gpui-component Library

The `gpui-component` crate provides higher-level UI components. Initialize in `main.rs`:

```rust
use gpui_component::Root;

Application::new().run(|cx: &mut App| {
    gpui_component::init(cx); // Must be first

    cx.open_window(options, |window, cx| {
        let view = cx.new(MainView::new);
        cx.new(|cx| Root::new(view, window, cx)) // Wrap in Root
    });
});
```

## Button

```rust
use gpui_component::button::{Button, ButtonVariants};

// Primary button
Button::new("save-btn")
    .primary()
    .label("Save")
    .on_click(cx.listener(|view, _, window, cx| {
        view.on_save(window, cx);
    }))

// Danger button
Button::new("delete-btn")
    .danger()
    .label("Delete")
    .on_click(cx.listener(|view, _, window, cx| {
        view.on_delete(window, cx);
    }))

// Ghost/subtle button
Button::new("cancel-btn")
    .ghost()
    .label("Cancel")
    .on_click(cx.listener(|view, _, _, cx| {
        view.close_dialog(cx);
    }))

// Compact button
Button::new("add-btn")
    .ghost()
    .compact()
    .label("+")
    .on_click(cx.listener(|view, _, _, cx| {
        view.add_item(cx);
    }))
```

## Input (Text Field)

```rust
use gpui_component::input::{Input, InputState};

// Create input state (Entity-based)
let input_state = cx.new(|cx| {
    InputState::new(window, cx)
        .placeholder("Enter branch name")
        .default_value("feature-")
});

// Store as Entity<InputState> in your view struct
self.branch_input = Some(input_state);

// Render
div().when_some(self.branch_input.as_ref(), |el, input| {
    el.child(Input::new(input.clone()))
})

// Read value
if let Some(input) = &self.branch_input {
    let value = input.read(cx).value().to_string();
}

// Focus the input
if let Some(input) = &self.branch_input {
    let handle = input.read(cx).focus_handle(cx).clone();
    window.focus(&handle);
}
```

## Theme Configuration

```rust
use gpui_component::theme::{Theme, ThemeConfig, ThemeConfigColors, ThemeMode};

let config = Rc::new(ThemeConfig {
    name: SharedString::from("My Theme"),
    mode: ThemeMode::Dark,
    is_default: true,
    font_family: Some("Inter".into()),
    mono_font_family: Some("JetBrains Mono".into()),
    font_size: Some(13.0),
    mono_font_size: Some(13.0),
    radius: Some(6),
    radius_lg: Some(8),
    shadow: Some(true),
    colors: my_colors(), // ThemeConfigColors
    highlight: None,
});

Theme::global_mut(cx).apply_config(&config);
```

### Theme Colors (JSON-based)

```rust
fn my_colors() -> ThemeConfigColors {
    let json = r#"{
        "background": "#0E1012",
        "foreground": "#B8C0CC",
        "border": "#2D3139",
        "primary.background": "#7CB4C8",
        "primary.foreground": "#F8FAFC",
        "danger.background": "#B87060",
        "success.background": "#6B8F5E"
    }"#;
    serde_json::from_str(json).unwrap_or_default()
}
```

## Root Component

All views must be wrapped in `Root` for gpui-component to function:

```rust
cx.new(|cx| Root::new(your_view, window, cx))
```

`Root` provides the theme context and base styling that all gpui-component widgets depend on.
