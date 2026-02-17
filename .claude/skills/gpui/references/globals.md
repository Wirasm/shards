# Global State

## Define

```rust
use gpui::Global;

#[derive(Clone)]
struct AppSettings {
    theme: Theme,
    language: String,
}

impl Global for AppSettings {}
```

## Set and Access

```rust
// Set once at startup
cx.set_global(AppSettings {
    theme: Theme::Dark,
    language: "en".into(),
});

// Read anywhere
let settings = cx.global::<AppSettings>();

// Update
cx.update_global::<AppSettings, _>(|settings, cx| {
    settings.theme = Theme::Light;
});
```

## When to Use

**Use Globals for:**
- App-wide config (theme, locale)
- Shared services (HTTP client, logger)
- Feature flags
- Read-mostly reference data

**Use Entities for:**
- Component state
- Frequently changing data
- State needing `cx.notify()` reactivity
- Anything requiring subscriptions/observations

## Key Notes

- `cx.update_global()` does NOT trigger automatic re-renders — you must `cx.notify()` manually on affected views
- Use `Arc` for shared resources inside globals (cheap to clone)
- One global per type — `cx.set_global::<T>()` overwrites any previous value of type `T`
