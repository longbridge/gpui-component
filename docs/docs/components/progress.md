---
title: Progress
description: Displays an indicator showing the completion progress of a task, typically displayed as a progress bar.
---

# Progress

A linear progress bar component that visually represents the completion percentage of a task. The progress bar features smooth animations, customizable colors, and automatic styling that adapts to the current theme.

## Import

```rust
use gpui_component::progress::Progress;
```

## Usage

### Basic Progress Bar

```rust
Progress::new("my-progress")
    .value(50.0) // 50% complete
```

### Different Progress Values

```rust
// Starting state (0%)
Progress::new("progress-0")
    .value(0.0)

// Partially complete (25%)
Progress::new("progress-25")
    .value(25.0)

// Nearly complete (75%)
Progress::new("progress-75")
    .value(75.0)

// Complete (100%)
Progress::new("progress-100")
    .value(100.0)
```

### Indeterminate State

```rust
// For unknown progress duration
Progress::new("indeterminate")
    .value(-1.0) // Any negative value shows as 0%

// Or explicitly set to 0 for starting state
Progress::new("starting")
    .value(0.0)
```

### Dynamic Progress Updates

```rust
struct MyComponent {
    progress_value: f32,
}

impl MyComponent {
    fn update_progress(&mut self, new_value: f32) {
        self.progress_value = new_value.clamp(0.0, 100.0);
    }

    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(
                h_flex()
                    .gap_2()
                    .child(Button::new("decrease").label("-").on_click(
                        cx.listener(|this, _, _, _| {
                            this.update_progress(this.progress_value - 10.0);
                        })
                    ))
                    .child(Button::new("increase").label("+").on_click(
                        cx.listener(|this, _, _, _| {
                            this.update_progress(this.progress_value + 10.0);
                        })
                    ))
            )
            .child(Progress::new("progress").value(self.progress_value))
            .child(format!("{}%", self.progress_value as i32))
    }
}
```

### File Upload Progress

```rust
struct FileUpload {
    bytes_uploaded: u64,
    total_bytes: u64,
}

impl FileUpload {
    fn progress_percentage(&self) -> f32 {
        if self.total_bytes == 0 {
            0.0
        } else {
            (self.bytes_uploaded as f32 / self.total_bytes as f32) * 100.0
        }
    }

    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_2()
            .child("Uploading file...")
            .child(Progress::new("upload-progress").value(self.progress_percentage()))
            .child(format!(
                "{} / {} bytes",
                self.bytes_uploaded,
                self.total_bytes
            ))
    }
}
```

### Loading States

```rust
struct LoadingComponent {
    is_loading: bool,
    progress: f32,
}

impl LoadingComponent {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .when(self.is_loading, |this| {
                this.child("Loading...")
                    .child(Progress::new("loading-progress").value(self.progress))
            })
            .when(!self.is_loading, |this| {
                this.child("Task completed!")
                    .child(Progress::new("loading-progress").value(100.0))
            })
    }
}
```

### Multi-Step Process

```rust
enum ProcessStep {
    Initializing,
    Processing,
    Finalizing,
    Complete,
}

struct MultiStepProcess {
    current_step: ProcessStep,
    step_progress: f32,
}

impl MultiStepProcess {
    fn overall_progress(&self) -> f32 {
        let base_progress = match self.current_step {
            ProcessStep::Initializing => 0.0,
            ProcessStep::Processing => 33.33,
            ProcessStep::Finalizing => 66.66,
            ProcessStep::Complete => 100.0,
        };

        if matches!(self.current_step, ProcessStep::Complete) {
            100.0
        } else {
            base_progress + (self.step_progress / 3.0)
        }
    }

    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(match self.current_step {
                ProcessStep::Initializing => "Initializing...",
                ProcessStep::Processing => "Processing data...",
                ProcessStep::Finalizing => "Finalizing...",
                ProcessStep::Complete => "Complete!",
            })
            .child(Progress::new("overall-progress").value(self.overall_progress()))
            .child(format!("{:.1}% complete", self.overall_progress()))
    }
}
```

## Examples

### Task Progress with Status

```rust
struct TaskProgress {
    completed_tasks: usize,
    total_tasks: usize,
}

impl TaskProgress {
    fn progress_value(&self) -> f32 {
        if self.total_tasks == 0 {
            0.0
        } else {
            (self.completed_tasks as f32 / self.total_tasks as f32) * 100.0
        }
    }

    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_2()
            .child(
                h_flex()
                    .justify_between()
                    .child("Task Progress")
                    .child(format!("{}/{}", self.completed_tasks, self.total_tasks))
            )
            .child(Progress::new("task-progress").value(self.progress_value()))
            .when(self.completed_tasks == self.total_tasks, |this| {
                this.child("All tasks completed!")
            })
    }
}
```

### Download Progress with Speed

```rust
struct DownloadProgress {
    downloaded: u64,
    total_size: u64,
    speed_mbps: f32,
}

impl DownloadProgress {
    fn eta_seconds(&self) -> u64 {
        if self.speed_mbps == 0.0 {
            0
        } else {
            let remaining_mb = (self.total_size - self.downloaded) as f32 / 1_000_000.0;
            (remaining_mb / self.speed_mbps) as u64
        }
    }

    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let progress = (self.downloaded as f32 / self.total_size as f32) * 100.0;

        v_flex()
            .gap_2()
            .child(
                h_flex()
                    .justify_between()
                    .child("Downloading...")
                    .child(format!("{:.1}%", progress))
            )
            .child(Progress::new("download-progress").value(progress))
            .child(
                h_flex()
                    .justify_between()
                    .child(format!("{:.1} MB/s", self.speed_mbps))
                    .child(format!("ETA: {}s", self.eta_seconds()))
            )
    }
}
```

### Installation Progress

```rust
struct InstallationProgress {
    current_package: String,
    package_index: usize,
    total_packages: usize,
    package_progress: f32,
}

impl InstallationProgress {
    fn overall_progress(&self) -> f32 {
        if self.total_packages == 0 {
            0.0
        } else {
            let completed_packages = self.package_index as f32;
            let current_package_contribution = self.package_progress / 100.0;
            let total_progress = (completed_packages + current_package_contribution)
                / self.total_packages as f32;
            total_progress * 100.0
        }
    }

    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child("Installing packages...")
            .child(
                v_flex()
                    .gap_2()
                    .child(
                        h_flex()
                            .justify_between()
                            .child(format!("Overall Progress"))
                            .child(format!("{}/{}", self.package_index + 1, self.total_packages))
                    )
                    .child(Progress::new("overall-progress").value(self.overall_progress()))
            )
            .child(
                v_flex()
                    .gap_2()
                    .child(format!("Installing: {}", self.current_package))
                    .child(Progress::new("package-progress").value(self.package_progress))
            )
    }
}
```

## Styling and Theming

The Progress component automatically adapts to the current theme:

### Theme Colors

```rust
// The progress bar uses theme colors automatically
// Background: theme.progress_bar with 20% opacity
// Fill: theme.progress_bar at full opacity

// These colors adapt to light/dark theme automatically
Progress::new("themed-progress").value(75.0) // Uses theme colors
```

### Visual Properties

- **Height**: 8px by default
- **Border Radius**: Matches theme radius (up to half the height)
- **Background**: Semi-transparent theme progress bar color (20% opacity)
- **Fill**: Full opacity theme progress bar color
- **Animation**: Smooth transitions when value changes
- **Corners**: Rounded on completion, left-rounded during progress

## Behavior Notes

- Values less than 0 are clamped to 0%
- Values greater than 100 are clamped to 100%
- Progress bar fills from left to right
- Border radius adjusts based on completion state:
  - Partial progress: Left side rounded only
  - Complete progress: Both sides rounded
- Background color is always a semi-transparent version of the fill color
- Height and radius adapt to theme settings automatically

## Best Practices

1. **Always provide text indicators** alongside the visual progress bar
2. **Use meaningful labels** to describe what is progressing
3. **Update progress regularly** but not too frequently to avoid performance issues
4. **Consider showing ETA or completion time** for long-running tasks
5. **Provide cancel/pause options** for lengthy operations
6. **Show final status** when progress reaches 100%
7. **Handle error states** gracefully with appropriate messaging
