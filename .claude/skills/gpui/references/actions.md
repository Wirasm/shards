# Actions and Keyboard Shortcuts

## Defining Actions

### Simple Actions (No Parameters)

```rust
use gpui::actions;

actions!(my_app, [MoveUp, MoveDown, Save, Quit, ToggleSidebar]);
```

### Actions with Parameters

```rust
use gpui::Action;
use serde::Deserialize;

#[derive(Clone, PartialEq, Action, Deserialize)]
#[action(namespace = editor)]
pub struct InsertText {
    pub text: String,
}

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = editor, no_json)]
pub struct Digit(pub u8);
```

## Binding Keys

```rust
const CONTEXT: &str = "Editor";

pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("up", MoveUp, Some(CONTEXT)),
        KeyBinding::new("down", MoveDown, Some(CONTEXT)),
        KeyBinding::new("cmd-s", Save, Some(CONTEXT)),
        KeyBinding::new("cmd-q", Quit, None), // Global (no context)
    ]);
}
```

### Key Format

```
"cmd-s"              // Command (macOS) / Ctrl (Win/Linux)
"ctrl-c"             // Control
"alt-f"              // Alt/Option
"shift-tab"          // Shift
"cmd-ctrl-f"         // Multiple modifiers
"a"-"z", "0"-"9"    // Letters/numbers
"f1"-"f12"           // Function keys
"up", "down", "left", "right", "enter", "escape", "space", "tab"
"backspace", "delete"
```

## Handling Actions

```rust
impl Render for Editor {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .key_context("Editor") // Must match binding context
            .on_action(cx.listener(Self::move_up))
            .on_action(cx.listener(Self::move_down))
            .on_action(cx.listener(Self::save))
            .child("Content")
    }
}

impl Editor {
    fn move_up(&mut self, _: &MoveUp, cx: &mut Context<Self>) {
        self.cursor -= 1;
        cx.notify();
    }

    fn save(&mut self, _: &Save, cx: &mut Context<Self>) {
        // Save logic
    }
}
```

## Context-Aware Bindings

Same key, different behavior depending on which element has focus:

```rust
cx.bind_keys([
    KeyBinding::new("escape", CloseModal, Some("Modal")),
    KeyBinding::new("escape", ClearSelection, Some("Editor")),
]);

// Set context on elements
div().key_context("Editor").child(editor_content)
div().key_context("Modal").child(modal_content)
```

## Raw Key Events

For keys that don't map to actions (e.g., terminal input):

```rust
div()
    .on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
        let key = event.keystroke.key.as_str();
        let cmd = event.keystroke.modifiers.platform;

        if cmd && key == "c" {
            view.copy(cx);
            return;
        }

        // Unhandled: let parent handle it
        cx.propagate();
    }))
```
