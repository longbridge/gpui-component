---
title: Chart
description: Beautiful charts and graphs for data visualization including line, bar, area, pie, and candlestick charts.
---

# Chart

A comprehensive charting library providing Line, Bar, Area, Pie, and Candlestick charts for data visualization. The charts feature smooth animations, customizable styling, tooltips, legends, and automatic theming that adapts to your application's theme.

## Import

```rust
use gpui_component::chart::{LineChart, BarChart, AreaChart, PieChart, CandlestickChart};
```

## Chart Types

### LineChart

A line chart displays data points connected by straight line segments, perfect for showing trends over time.

#### Basic Line Chart

```rust
#[derive(Clone)]
struct DataPoint {
    x: String,
    y: f64,
}

let data = vec![
    DataPoint { x: "Jan".to_string(), y: 100.0 },
    DataPoint { x: "Feb".to_string(), y: 150.0 },
    DataPoint { x: "Mar".to_string(), y: 120.0 },
];

LineChart::new(data)
    .x(|d| d.x.clone())
    .y(|d| d.y)
```

#### Line Chart Variants

```rust
// Basic curved line (default)
LineChart::new(data)
    .x(|d| d.month.clone())
    .y(|d| d.value)

// Linear interpolation
LineChart::new(data)
    .x(|d| d.month.clone())
    .y(|d| d.value)
    .linear()

// Step after interpolation
LineChart::new(data)
    .x(|d| d.month.clone())
    .y(|d| d.value)
    .step_after()

// With dots at data points
LineChart::new(data)
    .x(|d| d.month.clone())
    .y(|d| d.value)
    .dot()

// Custom stroke color
LineChart::new(data)
    .x(|d| d.month.clone())
    .y(|d| d.value)
    .stroke(cx.theme().success)
```

#### Tick Control

```rust
// Show every tick
LineChart::new(data)
    .x(|d| d.month.clone())
    .y(|d| d.value)
    .tick_margin(1)

// Show every 2nd tick
LineChart::new(data)
    .x(|d| d.month.clone())
    .y(|d| d.value)
    .tick_margin(2)
```

### BarChart

A bar chart uses rectangular bars to show comparisons among categories.

#### Basic Bar Chart

```rust
BarChart::new(data)
    .x(|d| d.category.clone())
    .y(|d| d.value)
```

#### Bar Chart Customization

```rust
// Custom fill colors
BarChart::new(data)
    .x(|d| d.category.clone())
    .y(|d| d.value)
    .fill(|d| d.color)

// With labels on bars
BarChart::new(data)
    .x(|d| d.category.clone())
    .y(|d| d.value)
    .label(|d| format!("{}", d.value))

// Custom tick spacing
BarChart::new(data)
    .x(|d| d.category.clone())
    .y(|d| d.value)
    .tick_margin(2)
```

### AreaChart

An area chart displays quantitative data visually, similar to a line chart but with the area below the line filled.

#### Basic Area Chart

```rust
AreaChart::new(data)
    .x(|d| d.time.clone())
    .y(|d| d.value)
```

#### Stacked Area Charts

```rust
// Multi-series area chart
AreaChart::new(data)
    .x(|d| d.date.clone())
    .y(|d| d.desktop)  // First series
    .stroke(cx.theme().chart_1)
    .fill(cx.theme().chart_1.opacity(0.4))
    .y(|d| d.mobile)   // Second series
    .stroke(cx.theme().chart_2)
    .fill(cx.theme().chart_2.opacity(0.4))
```

#### Area Chart Styling

```rust
use gpui::{linear_gradient, linear_color_stop};

// With gradient fill
AreaChart::new(data)
    .x(|d| d.month.clone())
    .y(|d| d.value)
    .fill(linear_gradient(
        0.,
        linear_color_stop(cx.theme().chart_1.opacity(0.4), 1.),
        linear_color_stop(cx.theme().background.opacity(0.3), 0.),
    ))

// Different interpolation styles
AreaChart::new(data)
    .x(|d| d.month.clone())
    .y(|d| d.value)
    .linear()  // or .step_after()
```

### PieChart

A pie chart displays data as slices of a circular chart, ideal for showing proportions.

#### Basic Pie Chart

```rust
PieChart::new(data)
    .value(|d| d.amount as f32)
    .outer_radius(100.)
```

#### Donut Chart

```rust
PieChart::new(data)
    .value(|d| d.amount as f32)
    .outer_radius(100.)
    .inner_radius(60.) // Creates donut effect
```

#### Pie Chart Customization

```rust
// Custom colors
PieChart::new(data)
    .value(|d| d.amount as f32)
    .outer_radius(100.)
    .color(|d| d.color)

// With padding between slices
PieChart::new(data)
    .value(|d| d.amount as f32)
    .outer_radius(100.)
    .inner_radius(60.)
    .pad_angle(4. / 100.) // 4% padding
```

### CandlestickChart

A candlestick chart displays financial data using OHLC (Open, High, Low, Close) values, perfect for visualizing stock prices and market trends.

#### Basic Candlestick Chart

```rust
#[derive(Clone)]
struct StockPrice {
    pub date: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
}

let data = vec![
    StockPrice { date: "Jan".to_string(), open: 100.0, high: 110.0, low: 95.0, close: 105.0 },
    StockPrice { date: "Feb".to_string(), open: 105.0, high: 115.0, low: 100.0, close: 112.0 },
    StockPrice { date: "Mar".to_string(), open: 112.0, high: 120.0, low: 108.0, close: 115.0 },
];

CandlestickChart::new(data)
    .x(|d| d.date.clone())
    .open(|d| d.open)
    .high(|d| d.high)
    .low(|d| d.low)
    .close(|d| d.close)
```

#### Candlestick Chart Customization

```rust
// Adjust body width ratio (default: 0.6)
CandlestickChart::new(data)
    .x(|d| d.date.clone())
    .open(|d| d.open)
    .high(|d| d.high)
    .low(|d| d.low)
    .close(|d| d.close)
    .body_width_ratio(0.4) // Narrower bodies

// Custom tick spacing
CandlestickChart::new(data)
    .x(|d| d.date.clone())
    .open(|d| d.open)
    .high(|d| d.high)
    .low(|d| d.low)
    .close(|d| d.close)
    .tick_margin(2) // Show every 2nd tick
```

#### Candlestick Chart Colors

The candlestick chart automatically uses theme colors:

- **Bullish** (close > open): `bullish` color (green)
- **Bearish** (close < open): `bearish` color (red)

## Data Structures

### Example Data Types

```rust
// Time series data
#[derive(Clone)]
struct DailyDevice {
    pub date: String,
    pub desktop: f64,
    pub mobile: f64,
}

// Category data with styling
#[derive(Clone)]
struct MonthlyDevice {
    pub month: String,
    pub desktop: f64,
    pub color_alpha: f32,
}

impl MonthlyDevice {
    pub fn color(&self, base_color: Hsla) -> Hsla {
        base_color.alpha(self.color_alpha)
    }
}

// Financial data
#[derive(Clone)]
struct StockPrice {
    pub date: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: u64,
}
```

## Chart Configuration

### Container Setup

```rust
fn chart_container(
    title: &str,
    chart: impl IntoElement,
    center: bool,
    cx: &mut Context<ChartStory>,
) -> impl IntoElement {
    v_flex()
        .flex_1()
        .h_full()
        .border_1()
        .border_color(cx.theme().border)
        .rounded_lg()
        .p_4()
        .child(
            div()
                .when(center, |this| this.text_center())
                .font_semibold()
                .child(title.to_string()),
        )
        .child(
            div()
                .when(center, |this| this.text_center())
                .text_color(cx.theme().muted_foreground)
                .text_sm()
                .child("Data period label"),
        )
        .child(div().flex_1().py_4().child(chart))
        .child(
            div()
                .when(center, |this| this.text_center())
                .font_semibold()
                .text_sm()
                .child("Summary statistic"),
        )
        .child(
            div()
                .when(center, |this| this.text_center())
                .text_color(cx.theme().muted_foreground)
                .text_sm()
                .child("Additional context"),
        )
}
```

### Theme Integration

```rust
// Charts automatically use theme colors
let chart = LineChart::new(data)
    .x(|d| d.date.clone())
    .y(|d| d.value)
    .stroke(cx.theme().chart_1); // Uses theme chart colors

// Available theme chart colors:
// cx.theme().chart_1
// cx.theme().chart_2
// cx.theme().chart_3
// ... up to chart_5
```

## API Reference

- [LineChart]
- [BarChart]
- [AreaChart]
- [PieChart]
- [CandlestickChart]

## Examples

### Sales Dashboard

```rust
#[derive(Clone)]
struct SalesData {
    month: String,
    revenue: f64,
    profit: f64,
    region: String,
}

fn sales_dashboard(data: Vec<SalesData>, cx: &mut Context<Self>) -> impl IntoElement {
    v_flex()
        .gap_4()
        .child(
            h_flex()
                .gap_4()
                .child(
                    chart_container(
                        "Monthly Revenue",
                        LineChart::new(data.clone())
                            .x(|d| d.month.clone())
                            .y(|d| d.revenue)
                            .stroke(cx.theme().chart_1)
                            .dot(),
                        false,
                        cx,
                    )
                )
                .child(
                    chart_container(
                        "Profit Breakdown",
                        PieChart::new(data.clone())
                            .value(|d| d.profit as f32)
                            .outer_radius(80.)
                            .color(|d| match d.region.as_str() {
                                "North" => cx.theme().chart_1,
                                "South" => cx.theme().chart_2,
                                "East" => cx.theme().chart_3,
                                "West" => cx.theme().chart_4,
                                _ => cx.theme().chart_5,
                            }),
                        true,
                        cx,
                    )
                )
        )
        .child(
            chart_container(
                "Regional Performance",
                BarChart::new(data)
                    .x(|d| d.region.clone())
                    .y(|d| d.revenue)
                    .fill(|d| match d.region.as_str() {
                        "North" => cx.theme().chart_1,
                        "South" => cx.theme().chart_2,
                        "East" => cx.theme().chart_3,
                        "West" => cx.theme().chart_4,
                        _ => cx.theme().chart_5,
                    })
                    .label(|d| format!("${:.0}k", d.revenue / 1000.)),
                false,
                cx,
            )
        )
}
```

### Multi-Series Time Chart

```rust
#[derive(Clone)]
struct DeviceUsage {
    date: String,
    desktop: f64,
    mobile: f64,
    tablet: f64,
}

fn device_usage_chart(data: Vec<DeviceUsage>, cx: &mut Context<Self>) -> impl IntoElement {
    chart_container(
        "Device Usage Over Time",
        AreaChart::new(data)
            .x(|d| d.date.clone())
            .y(|d| d.desktop)
            .stroke(cx.theme().chart_1)
            .fill(linear_gradient(
                0.,
                linear_color_stop(cx.theme().chart_1.opacity(0.4), 1.),
                linear_color_stop(cx.theme().background.opacity(0.3), 0.),
            ))
            .y(|d| d.mobile)
            .stroke(cx.theme().chart_2)
            .fill(linear_gradient(
                0.,
                linear_color_stop(cx.theme().chart_2.opacity(0.4), 1.),
                linear_color_stop(cx.theme().background.opacity(0.3), 0.),
            ))
            .y(|d| d.tablet)
            .stroke(cx.theme().chart_3)
            .fill(linear_gradient(
                0.,
                linear_color_stop(cx.theme().chart_3.opacity(0.4), 1.),
                linear_color_stop(cx.theme().background.opacity(0.3), 0.),
            ))
            .tick_margin(3),
        false,
        cx,
    )
}
```

### Financial Chart

```rust
#[derive(Clone)]
struct StockData {
    date: String,
    price: f64,
    volume: u64,
}

#[derive(Clone)]
struct StockOHLC {
    date: String,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
}

fn stock_chart(ohlc_data: Vec<StockOHLC>, price_data: Vec<StockData>, cx: &mut Context<Self>) -> impl IntoElement {
    v_flex()
        .gap_4()
        .child(
            chart_container(
                "Stock Price - Candlestick",
                CandlestickChart::new(ohlc_data.clone())
                    .x(|d| d.date.clone())
                    .open(|d| d.open)
                    .high(|d| d.high)
                    .low(|d| d.low)
                    .close(|d| d.close)
                    .tick_margin(3),
                false,
                cx,
            )
        )
        .child(
            chart_container(
                "Stock Price - Line",
                LineChart::new(price_data.clone())
                    .x(|d| d.date.clone())
                    .y(|d| d.price)
                    .stroke(cx.theme().chart_1)
                    .linear()
                    .tick_margin(5),
                false,
                cx,
            )
        )
        .child(
            chart_container(
                "Trading Volume",
                BarChart::new(price_data)
                    .x(|d| d.date.clone())
                    .y(|d| d.volume as f64)
                    .fill(|d| {
                        if d.volume > 1000000 {
                            cx.theme().chart_1
                        } else {
                            cx.theme().muted_foreground.opacity(0.6)
                        }
                    })
                    .tick_margin(5),
                false,
                cx,
            )
        )
}
```

## Customization Options

### Color Schemes

```rust
// Theme-based colors (recommended)
LineChart::new(data)
    .x(|d| d.x.clone())
    .y(|d| d.y)
    .stroke(cx.theme().chart_1)

// Custom color palette
let colors = [
    cx.theme().success,
    cx.theme().warning,
    cx.theme().destructive,
    cx.theme().info,
    cx.theme().chart_1,
];

BarChart::new(data)
    .x(|d| d.category.clone())
    .y(|d| d.value)
    .fill(|d| colors[d.category_index % colors.len()])
```

### Responsive Design

```rust
// Container with responsive sizing
div()
    .flex_1()
    .min_h(px(300.))
    .max_h(px(600.))
    .w_full()
    .child(
        LineChart::new(data)
            .x(|d| d.x.clone())
            .y(|d| d.y)
    )
```

### Grid and Axis Styling

Charts automatically include:

- Grid lines with dashed appearance
- X-axis labels with smart positioning
- Y-axis scaling starting from zero
- Responsive tick spacing based on `tick_margin`

## Performance Considerations

### Large Datasets

```rust
// For large datasets, consider data sampling
let sampled_data: Vec<_> = data
    .iter()
    .step_by(5) // Show every 5th point
    .cloned()
    .collect();

LineChart::new(sampled_data)
    .x(|d| d.date.clone())
    .y(|d| d.value)
    .tick_margin(3) // Reduce tick density
```

### Memory Optimization

```rust
// Use efficient data accessors
LineChart::new(data)
    .x(|d| d.date.clone()) // Clone only when necessary
    .y(|d| d.value)        // Direct field access
```

## Integration Examples

### With State Management

```rust
struct ChartComponent {
    data: Vec<DataPoint>,
    chart_type: ChartType,
    time_range: TimeRange,
}

impl ChartComponent {
    fn render_chart(&self, cx: &mut Context<Self>) -> impl IntoElement {
        match self.chart_type {
            ChartType::Line => LineChart::new(self.filtered_data())
                .x(|d| d.date.clone())
                .y(|d| d.value)
                .into_any_element(),
            ChartType::Bar => BarChart::new(self.filtered_data())
                .x(|d| d.date.clone())
                .y(|d| d.value)
                .into_any_element(),
            ChartType::Area => AreaChart::new(self.filtered_data())
                .x(|d| d.date.clone())
                .y(|d| d.value)
                .into_any_element(),
        }
    }

    fn filtered_data(&self) -> Vec<DataPoint> {
        self.data
            .iter()
            .filter(|d| self.time_range.contains(&d.date))
            .cloned()
            .collect()
    }
}
```

### Real-time Updates

```rust
struct LiveChart {
    data: Vec<DataPoint>,
    max_points: usize,
}

impl LiveChart {
    fn add_data_point(&mut self, point: DataPoint) {
        self.data.push(point);
        if self.data.len() > self.max_points {
            self.data.remove(0); // Remove oldest point
        }
    }

    fn render(&self, cx: &mut Context<Self>) -> impl IntoElement {
        LineChart::new(self.data.clone())
            .x(|d| d.timestamp.clone())
            .y(|d| d.value)
            .linear()
            .dot()
    }
}
```

[LineChart]: https://docs.rs/gpui-component/latest/gpui_component/chart/struct.LineChart.html
[BarChart]: https://docs.rs/gpui-component/latest/gpui_component/chart/struct.BarChart.html
[AreaChart]: https://docs.rs/gpui-component/latest/gpui_component/chart/struct.AreaChart.html
[PieChart]: https://docs.rs/gpui-component/latest/gpui_component/chart/struct.PieChart.html
[CandlestickChart]: https://docs.rs/gpui-component/latest/gpui_component/chart/struct.CandlestickChart.html
