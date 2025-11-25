---
title: WebView
description: Embeds web content using a native webview control for displaying HTML, loading URLs, and executing JavaScript.
---

# WebView

The WebView component provides a native web browser control that can display web content within your GPUI application. It's built on top of the WRY (WebView Rendering library for YHYH) crate and supports loading URLs, rendering HTML content, executing JavaScript, and handling navigation.

## Feature Flag

WebView requires the optional `webview` feature to be enabled:

```toml-vue
[dependencies]
gpui-component = { version = "{{ VERSION }}", features = ["webview"] }
```

## Import

```rust
use gpui_component::webview::WebView;
use gpui_component::wry; // Re-exported WRY crate
```

## Platform Support

WebView supports multiple platforms with platform-specific implementations:

- **Windows**: Uses WebView2 (EdgeHTML/Chromium)
- **macOS**: Uses WKWebView
- **Linux**: Uses WebKitGTK
- **iOS/Android**: Platform-specific webview controls

## Usage

### Basic WebView

```rust
use gpui_component::webview::WebView;
use gpui_component::wry;

// Create a WebView with WRY WebViewBuilder
let webview = cx.new(|cx| {
    let builder = wry::WebViewBuilder::new();

    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "ios", target_os = "android"))]
    let webview = {
        use raw_window_handle::HasWindowHandle;
        let window_handle = window.window_handle().expect("No window handle");
        builder.build_as_child(&window_handle).unwrap()
    };

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "ios", target_os = "android")))]
    let webview = {
        use gtk::prelude::*;
        use wry::WebViewBuilderExtUnix;
        let fixed = gtk::Fixed::builder().build();
        fixed.show_all();
        builder.build_gtk(&fixed).unwrap()
    };

    WebView::new(webview, window, cx)
});
```

### Loading URLs

```rust
// Load a URL
webview.update(cx, |webview, _| {
    webview.load_url("https://example.com");
});

// Load Google (common example)
webview.update(cx, |webview, _| {
    webview.load_url("https://google.com");
});
```

### Navigation Control

```rust
// Go back in browser history
webview.update(cx, |webview, _| {
    webview.back().unwrap();
});

// Navigate programmatically using JavaScript
webview.update(cx, |webview, _| {
    webview.evaluate_script("window.location.href = 'https://example.com'").unwrap();
});
```

### Visibility Control

```rust
// Show the webview
webview.update(cx, |webview, _| {
    webview.show();
});

// Hide the webview
webview.update(cx, |webview, _| {
    webview.hide();
});

// Check visibility status
let is_visible = webview.read(cx).visible();
```

### JavaScript Execution

```rust
// Execute JavaScript code
webview.update(cx, |webview, _| {
    webview.evaluate_script("console.log('Hello from GPUI!')").unwrap();
});

// Navigate using JavaScript
webview.update(cx, |webview, _| {
    webview.evaluate_script("history.back()").unwrap();
});

// Manipulate DOM
webview.update(cx, |webview, _| {
    webview.evaluate_script(r#"
        document.body.style.backgroundColor = 'lightblue';
        document.title = 'Modified by GPUI';
    "#).unwrap();
});
```

### Browser-like Interface

```rust
use gpui_component::input::{InputState, Input};

pub struct WebBrowser {
    webview: Entity<WebView>,
    address_input: Entity<InputState>,
}

impl WebBrowser {
    pub fn new(window: &mut Window, cx: &mut App) -> Entity<Self> {
        let webview = Self::create_webview(window, cx);
        let address_input = cx.new(|cx| {
            InputState::new(window, cx).default_value("https://google.com")
        });

        // Load initial URL
        let url = address_input.read(cx).value().clone();
        webview.update(cx, |view, _| view.load_url(&url));

        cx.new(|cx| {
            let browser = Self { webview, address_input: address_input.clone() };

            // Subscribe to address bar changes
            cx.subscribe(&address_input, |this: &mut Self, input, event: &InputEvent, cx| {
                match event {
                    InputEvent::PressEnter { .. } => {
                        let url = input.read(cx).value().clone();
                        this.webview.update(cx, |view, _| view.load_url(&url));
                    }
                    _ => {}
                }
            }).detach();

            browser
        })
    }
}
```

### Custom WebView Configuration

```rust
// Configure WebView with custom settings
let builder = wry::WebViewBuilder::new()
    .with_url("https://example.com")
    .with_initialization_script("console.log('WebView initialized')")
    .with_user_agent("MyApp/1.0")
    .with_devtools(true); // Enable developer tools

let webview = builder.build_as_child(&window_handle).unwrap();
let webview_component = WebView::new(webview, window, cx);
```

### Handling Mouse Events

```rust
// WebView automatically handles mouse events within its bounds
// Clicking outside the webview will blur focus
div()
    .size_full()
    .child(webview)
    .on_mouse_down(|event, _, _| {
        // Handle clicks outside webview
        println!("Clicked at: {:?}", event.position);
    })
```

## API Reference

### WebView

| Method                     | Description                           |
| -------------------------- | ------------------------------------- |
| `new(webview, window, cx)` | Create a new WebView from WRY WebView |
| `load_url(url)`            | Load a URL in the webview             |
| `show()`                   | Make the webview visible              |
| `hide()`                   | Hide the webview                      |
| `visible()`                | Check if webview is visible           |
| `bounds()`                 | Get webview bounds                    |
| `back()`                   | Navigate back in history              |

### WebViewElement

| Method                          | Description                  |
| ------------------------------- | ---------------------------- |
| `new(view, parent, window, cx)` | Create a new webview element |

### WRY Integration

The WebView component wraps the WRY WebView, providing access to all WRY methods through `Deref`:

```rust
// Access WRY methods directly
webview.evaluate_script("console.log('test')").unwrap();
webview.load_url("https://example.com").unwrap();
webview.set_bounds(rect).unwrap();
```

## Examples

### Simple Web Browser

```rust
use gpui_component::{
    webview::WebView,
    input::{Input, InputState, InputEvent},
    h_flex, v_flex
};

struct SimpleBrowser {
    webview: Entity<WebView>,
    address_bar: Entity<InputState>,
}

impl Render for SimpleBrowser {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .child(
                h_flex()
                    .p_2()
                    .gap_2()
                    .child("Address:")
                    .child(Input::new(&self.address_bar))
            )
            .child(
                div()
                    .flex_1()
                    .border_1()
                    .border_color(cx.theme().border)
                    .child(self.webview.clone())
            )
    }
}
```

### WebView with Navigation Controls

```rust
struct WebBrowserWithControls {
    webview: Entity<WebView>,
    can_go_back: bool,
}

impl Render for WebBrowserWithControls {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .child(
                h_flex()
                    .p_2()
                    .gap_2()
                    .child(
                        Button::new("back")
                            .label("← Back")
                            .disabled(!self.can_go_back)
                            .on_click(|_, _, cx| {
                                // Handle back navigation
                            })
                    )
                    .child(
                        Button::new("refresh")
                            .label("⟳ Refresh")
                            .on_click(|_, _, cx| {
                                // Handle refresh
                            })
                    )
            )
            .child(
                div()
                    .flex_1()
                    .child(self.webview.clone())
            )
    }
}
```

### Loading Local HTML Content

```rust
// Load local HTML content
let html_content = r#"
<!DOCTYPE html>
<html>
<head>
    <title>Local Content</title>
    <style>
        body { font-family: Arial, sans-serif; padding: 20px; }
        .highlight { background-color: yellow; }
    </style>
</head>
<body>
    <h1>Hello from Local HTML</h1>
    <p class="highlight">This content is loaded locally!</p>
    <script>
        console.log('Local HTML loaded successfully');
    </script>
</body>
</html>
"#;

webview.update(cx, |webview, _| {
    webview.load_html(html_content).unwrap();
});
```

## Platform-Specific Considerations

### Windows

- Requires WebView2 runtime to be installed
- Uses EdgeHTML/Chromium engine
- Supports modern web standards

### macOS

- Uses WKWebView (modern WebKit)
- Excellent performance and standards compliance
- Automatic memory management

### Linux

- Requires WebKitGTK development libraries
- May need GTK initialization: `gtk::init().unwrap()`
- Consider distributing with required libraries

### Initialization Example

```rust
pub fn init_webview(_: &mut App) {
    #[cfg(target_os = "linux")]
    gtk::init().unwrap();
}
```

## Security Considerations

### Content Security

- **Validate URLs**: Always validate and sanitize URLs before loading
- **HTTPS Preferred**: Use HTTPS URLs when possible for secure content
- **Origin Control**: Be careful when loading external content

```rust
fn load_safe_url(webview: &mut WebView, url: &str) -> Result<(), &'static str> {
    if url.starts_with("https://") || url.starts_with("http://localhost") {
        webview.load_url(url);
        Ok(())
    } else {
        Err("Only HTTPS and localhost URLs allowed")
    }
}
```

### JavaScript Security

- **Sanitize Scripts**: Validate JavaScript before execution
- **Limit Scope**: Restrict JavaScript access to necessary APIs only
- **Error Handling**: Always handle JavaScript execution errors

```rust
fn execute_safe_script(webview: &mut WebView, script: &str) -> anyhow::Result<()> {
    // Validate script content
    if script.contains("eval(") || script.contains("Function(") {
        return Err(anyhow::anyhow!("Potentially unsafe script"));
    }

    webview.evaluate_script(script)
}
```

## Performance Tips

### Memory Management

- WebView automatically cleans up on drop
- Hide webviews when not visible to save resources
- Consider lazy loading for multiple webviews

### Rendering Optimization

- Set appropriate bounds for better performance
- Use content masks for complex layouts
- Minimize frequent size changes

```rust
// Efficient webview sizing
impl WebView {
    fn set_optimal_size(&mut self, bounds: Bounds<Pixels>) {
        if self.bounds != bounds {
            self.bounds = bounds;
            self.webview.set_bounds(/* convert bounds */).unwrap();
        }
    }
}
```

## Common Patterns

### Conditional Rendering

```rust
// Only render webview when needed
if should_show_webview {
    div().child(webview)
} else {
    div().child("WebView hidden")
}
```

### State Management

```rust
// Track webview state
struct WebViewState {
    current_url: String,
    is_loading: bool,
    can_go_back: bool,
    can_go_forward: bool,
}
```

## Troubleshooting

### Common Issues

1. **WebView not appearing**: Check platform-specific initialization
2. **URLs not loading**: Verify network connectivity and URL format
3. **JavaScript errors**: Check browser developer tools if available
4. **Focus issues**: Ensure proper focus handling in parent container

### Debug Tips

```rust
// Enable debug logging
webview.evaluate_script(r#"
    console.log('WebView ready');
    window.addEventListener('error', (e) => {
        console.error('WebView error:', e);
    });
"#).unwrap();
```
