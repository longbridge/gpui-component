---
title: Chart
description: 支持折线图、柱状图、面积图、饼图和 K 线图的数据可视化组件。
---

# Chart

Chart 是一组完整的数据可视化组件，提供 Line、Bar、Area、Pie 和 Candlestick 图表。它们支持动画、自定义样式、主题配色和多种展示方式，适合仪表盘、统计分析和行情场景。

## 导入

```rust
use gpui_component::chart::{LineChart, BarChart, AreaChart, PieChart, CandlestickChart};
```

## 图表类型

### LineChart

折线图用于展示随时间变化的趋势。

#### 基础折线图

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

#### 折线图变体

```rust
LineChart::new(data)
    .x(|d| d.month.clone())
    .y(|d| d.value)

LineChart::new(data)
    .x(|d| d.month.clone())
    .y(|d| d.value)
    .linear()

LineChart::new(data)
    .x(|d| d.month.clone())
    .y(|d| d.value)
    .step_after()

LineChart::new(data)
    .x(|d| d.month.clone())
    .y(|d| d.value)
    .dot()

LineChart::new(data)
    .x(|d| d.month.clone())
    .y(|d| d.value)
    .stroke(cx.theme().success)
```

#### 刻度控制

```rust
LineChart::new(data)
    .x(|d| d.month.clone())
    .y(|d| d.value)
    .tick_margin(1)

LineChart::new(data)
    .x(|d| d.month.clone())
    .y(|d| d.value)
    .tick_margin(2)
```

### BarChart

柱状图通过矩形条形对比不同类别的数据，并可通过 `alignment` 选项切换垂直或水平方向。

#### 基础柱状图

```rust
BarChart::new(data)
    .band(|d| d.category.clone())
    .value(|d| d.value)
```

#### 自定义柱状图

```rust
// 自定义填充颜色
//
// `fill` 闭包接收四个参数：数据项、柱子的像素边界（相对于图表原点）、
// 图表的像素边界，以及柱子的 `BarAlignment`。返回值可以是任何能转换为
// `Background` 的类型（纯色、渐变、图案等）。
BarChart::new(data)
    .band(|d| d.category.clone())
    .value(|d| d.value)
    .fill(|d, _bar_bounds, _chart_bounds, _alignment| d.color)

// 显示数值标签
BarChart::new(data)
    .band(|d| d.category.clone())
    .value(|d| d.value)
    .label(|d| format!("{}", d.value))

// 自定义刻度间距
BarChart::new(data)
    .band(|d| d.category.clone())
    .value(|d| d.value)
    .tick_margin(2)

// 隐藏分类轴的轴线和标签
BarChart::new(data)
    .band(|d| d.category.clone())
    .value(|d| d.value)
    .label_axis(false)
```

#### 柱状图渐变填充

如需让渐变方向跟随柱子方向，请使用 `fill_gradient`。闭包接收三个参数：数据项、图表的完整数据范围（`chart_range`），以及一个 `chart_to_bar` 辅助函数（将图表数值坐标映射为柱子局部的渐变位置，其中 `0.0` 表示柱子的基线端，`1.0` 表示尖端）。渐变方向由柱子的 `BarAlignment` 推导，使 stop-0 始终位于基线端、stop-1 位于尖端。

```rust
use gpui::linear_color_stop;

// 单柱渐变：每个柱子都从半透明基线渐变到完全不透明的尖端，
// 与该柱子的具体数值无关。
BarChart::new(data)
    .band(|d| d.category.clone())
    .value(|d| d.value)
    .fill_gradient(|d, _chart_range, _chart_to_bar| {
        let c = d.color;
        [
            linear_color_stop(c.opacity(0.3), 0.0),
            linear_color_stop(c, 1.0),
        ]
    })

// 跨图表渐变：每根柱子展示同一条覆盖整个图表数值范围的渐变中
// 对应自身值域的那一段。超出 `[0, 1]` 的 stop 会被裁剪到柱子内，
// 颜色会在裁剪点处插值，使整体效果保持连续。
BarChart::new(data)
    .band(|d| d.category.clone())
    .value(|d| d.value)
    .fill_gradient(|d, chart_range, chart_to_bar| {
        let c = d.color;
        [
            linear_color_stop(c.opacity(0.3), chart_to_bar(*chart_range.start())),
            linear_color_stop(c,              chart_to_bar(*chart_range.end())),
        ]
    })
```

`fill` 与 `fill_gradient` 互斥——设置其中一个会清空另一个。

#### 柱状图对齐方式

`BarAlignment` 用于控制柱子的方向以及基线所在的一侧，需从 `gpui_component::plot::shape` 导入。

```rust
use gpui_component::plot::shape::BarAlignment;

// 默认：垂直方向 - 向上
BarChart::new(data)
    .band(|d| d.category.clone())
    .value(|d| d.value)
    .alignment(BarAlignment::Bottom)

// 垂直方向 - 向下
BarChart::new(data)
    .band(|d| d.category.clone())
    .value(|d| d.value)
    .alignment(BarAlignment::Top)

// 水平方向 - 向右
BarChart::new(data)
    .band(|d| d.category.clone())
    .value(|d| d.value)
    .alignment(BarAlignment::Left)

// 水平方向 - 向左
BarChart::new(data)
    .band(|d| d.category.clone())
    .value(|d| d.value)
    .alignment(BarAlignment::Right)
```

#### 柱状图圆角

为柱状条形设置圆角。可传入任意可转换为 `Corners<Pixels>` 的值——
使用单个 `px(..)` 表示四角统一圆角，或手动构造 `Corners`
仅对特定角进行圆角处理（例如仅对柱顶一端进行圆角）。

```rust
use gpui::{px, Corners};

// 所有柱条统一 4px 圆角
BarChart::new(data)
    .band(|d| d.category.clone())
    .value(|d| d.value)
    .corner_radii(px(4.))

// 仅顶部圆角（适用于底部对齐柱状图的柱顶一端）
BarChart::new(data)
    .band(|d| d.category.clone())
    .value(|d| d.value)
    .corner_radii(Corners {
        top_left: px(4.),
        top_right: px(4.),
        bottom_left: px(0.),
        bottom_right: px(0.),
    })
```

### AreaChart

面积图类似折线图，但会填充曲线下方的区域。

#### 基础面积图

```rust
AreaChart::new(data)
    .x(|d| d.time.clone())
    .y(|d| d.value)
```

#### 多系列面积图

```rust
AreaChart::new(data)
    .x(|d| d.date.clone())
    .y(|d| d.desktop)
    .stroke(cx.theme().chart_1)
    .fill(cx.theme().chart_1.opacity(0.4))
    .y(|d| d.mobile)
    .stroke(cx.theme().chart_2)
    .fill(cx.theme().chart_2.opacity(0.4))
```

#### 样式

```rust
use gpui::{linear_gradient, linear_color_stop};

AreaChart::new(data)
    .x(|d| d.month.clone())
    .y(|d| d.value)
    .fill(linear_gradient(
        0.,
        linear_color_stop(cx.theme().chart_1.opacity(0.4), 1.),
        linear_color_stop(cx.theme().background.opacity(0.3), 0.),
    ))

AreaChart::new(data)
    .x(|d| d.month.clone())
    .y(|d| d.value)
    .linear()
```

### PieChart

饼图适合展示占比关系。

#### 基础饼图

```rust
PieChart::new(data)
    .value(|d| d.amount as f32)
    .outer_radius(100.)
```

#### 环形图

```rust
PieChart::new(data)
    .value(|d| d.amount as f32)
    .outer_radius(100.)
    .inner_radius(60.)
```

#### 自定义

```rust
PieChart::new(data)
    .value(|d| d.amount as f32)
    .outer_radius(100.)
    .color(|d| d.color)

PieChart::new(data)
    .value(|d| d.amount as f32)
    .outer_radius(100.)
    .inner_radius(60.)
    .pad_angle(4. / 100.)
```

### CandlestickChart

K 线图适合展示金融行情中的 OHLC 数据。

#### 基础 K 线图

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

#### 自定义

```rust
CandlestickChart::new(data)
    .x(|d| d.date.clone())
    .open(|d| d.open)
    .high(|d| d.high)
    .low(|d| d.low)
    .close(|d| d.close)
    .body_width_ratio(0.4)

CandlestickChart::new(data)
    .x(|d| d.date.clone())
    .open(|d| d.open)
    .high(|d| d.high)
    .low(|d| d.low)
    .close(|d| d.close)
    .tick_margin(2)
```

涨跌颜色会自动使用主题中的 bullish 和 bearish 配色。

## 数据结构示例

```rust
#[derive(Clone)]
struct DailyDevice {
    pub date: String,
    pub desktop: f64,
    pub mobile: f64,
}

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

## 图表配置

### 容器布局

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
        .rounded(cx.theme().radius_lg)
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

### 主题集成

```rust
let chart = LineChart::new(data)
    .x(|d| d.date.clone())
    .y(|d| d.value)
    .stroke(cx.theme().chart_1);
```

可用主题色通常包括 `chart_1` 到 `chart_5`。

## API 参考

- [LineChart]
- [BarChart]
- [AreaChart]
- [PieChart]
- [CandlestickChart]

## 示例

### 销售仪表盘

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
                    .band(|d| d.region.clone())
                    .value(|d| d.revenue)
                    .fill(|d, _, _, _| match d.region.as_str() {
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

### 多系列时间图

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

### 金融图表

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
                    .band(|d| d.date.clone())
                    .value(|d| d.volume as f64)
                    .fill(|d, _, _, _| {
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

## 自定义选项

### 配色

```rust
LineChart::new(data)
    .x(|d| d.x.clone())
    .y(|d| d.y)
    .stroke(cx.theme().chart_1)

let colors = [
    cx.theme().success,
    cx.theme().warning,
    cx.theme().destructive,
    cx.theme().info,
    cx.theme().chart_1,
];

BarChart::new(data)
    .band(|d| d.category.clone())
    .value(|d| d.value)
    .fill(|d, _, _, _| colors[d.category_index % colors.len()])
```

### 响应式容器

```rust
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

### 默认样式

图表默认会自动包含：

- 虚线网格
- 自动定位的 X 轴标签
- 从 0 开始的 Y 轴刻度
- 基于 `tick_margin` 的刻度稀疏控制

## 性能建议

### 大数据集

```rust
let sampled_data: Vec<_> = data
    .iter()
    .step_by(5)
    .cloned()
    .collect();

LineChart::new(sampled_data)
    .x(|d| d.date.clone())
    .y(|d| d.value)
    .tick_margin(3)
```

### 内存优化

```rust
LineChart::new(data)
    .x(|d| d.date.clone())
    .y(|d| d.value)
```

## 集成示例

### 结合状态管理

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
                .band(|d| d.date.clone())
                .value(|d| d.value)
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

### 实时更新

```rust
struct LiveChart {
    data: Vec<DataPoint>,
    max_points: usize,
}

impl LiveChart {
    fn add_data_point(&mut self, point: DataPoint) {
        self.data.push(point);
        if self.data.len() > self.max_points {
            self.data.remove(0);
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
