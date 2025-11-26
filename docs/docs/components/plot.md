---
title: Plot
description: A low-level plotting library for creating custom charts and data visualizations.
---

# Plot

The `plot` module provides low-level building blocks for creating custom charts. It includes scales, shapes, and utilities that power the high-level `Chart` components.

## Import

```rust
use gpui_component::plot::{
    scale::{Scale, ScaleLinear, ScaleBand, ScaleOrdinal},
    shape::{Bar, Stack},
    PlotAxis, AxisText
};
```

## Scales

Scales map a dimension of abstract data to a visual representation.

### ScaleLinear

Maps a continuous quantitative domain to a continuous range.

```rust
let scale = ScaleLinear::new(
    vec![0., 100.],   // Domain (data values)
    vec![0., 500.]    // Range (pixel coordinates)
);

scale.tick(&50.); // Returns pixel position
```

### ScaleBand

Maps a discrete domain to a continuous range, useful for bar charts.

```rust
let scale = ScaleBand::new(
    vec!["A", "B", "C"], // Domain
    vec![0., 300.]       // Range
)
.padding_inner(0.1)
.padding_outer(0.1);

scale.band_width(); // Returns width of each band
scale.tick(&"A");   // Returns start position of band "A"
```

### ScaleOrdinal

Maps a discrete domain to a discrete range.

```rust
let scale = ScaleOrdinal::new(
    vec!["A", "B", "C"], // Domain
    vec![color1, color2, color3] // Range
);

scale.map(&"A"); // Returns color1
```

## Shapes

### Bar

Renders a bar shape, commonly used in bar charts.

```rust
Bar::new()
    .data(data)
    .band_width(30.)
    .x(|d| x_scale.tick(&d.category))
    .y0(|d| y_scale.tick(&0.).unwrap())
    .y1(|d| y_scale.tick(&d.value))
    .fill(|d| color_scale.map(&d.category))
    .paint(&bounds, window, cx);
```

#### Stacked Bar

Supports stacked data rendering.

```rust
Bar::new()
    .stack_data(&series) // Pass StackSeries data
    .band_width(30.)
    .x(|d| x_scale.tick(&d.data.date))
    // y0/y1 are automatically handled from stack data
    .fill(|_| color)
    .paint(&bounds, window, cx);
```

### Stack

Computes stacked layout for data series.

```rust
let stack = Stack::new()
    .data(data)
    .keys(vec!["series1", "series2"])
    .value(|d, key| match key {
        "series1" => Some(d.val1),
        "series2" => Some(d.val2),
        _ => None
    });

let series = stack.series(); // Returns Vec<StackSeries<T>>
```

## Components

### PlotAxis

Renders chart axes with labels and ticks.

```rust
PlotAxis::new()
    .x(height) // Y position for X axis
    .x_label(labels) // Iterator of AxisText
    .stroke(cx.theme().border)
    .paint(&bounds, window, cx);
```

## Examples

### Custom Bar Chart Implementation

Here's how to implement a custom stacked bar chart using low-level plot primitives:

```rust
struct StackedBarChart {
    data: Vec<DailyDevice>,
    series: Vec<StackSeries<DailyDevice>>,
}

impl StackedBarChart {
    pub fn new(data: Vec<DailyDevice>) -> Self {
        let series = Stack::new()
            .data(data.clone())
            .keys(vec!["desktop", "mobile"])
            .value(|d, key| match key {
                "desktop" => Some(d.desktop),
                "mobile" => Some(d.mobile),
                _ => None,
            })
            .series();

        Self { data, series }
    }
}

impl Plot for StackedBarChart {
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        // 1. Setup Scales
        let x = ScaleBand::new(
            self.data.iter().map(|v| v.date.clone()).collect(),
            vec![0., width],
        );
        
        let y = ScaleLinear::new(vec![0., max_value], vec![height, 0.]);

        // 2. Draw Axis
        // ... (axis rendering logic)

        // 3. Draw Stacked Bars
        let bar = Bar::new()
            .stack_data(&self.series)
            .band_width(x.band_width())
            .x(move |d| x.tick(&d.data.date))
            .fill(move |_| cx.theme().chart_1);

        bar.paint(&bounds, window, cx);
    }
}
```
