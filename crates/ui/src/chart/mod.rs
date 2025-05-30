mod area_chart;
mod bar_chart;
mod line_chart;
mod pie_chart;

pub use area_chart::AreaChart;
pub use bar_chart::BarChart;
pub use line_chart::LineChart;
pub use pie_chart::{PieChart, PieChartDelegate};

use gpui::SharedString;
use num_traits::{Num, ToPrimitive};

use crate::plot::scale::Sealed;

pub trait ChartDelegate<X, Y>
where
    X: PartialEq + Into<SharedString>,
    Y: Copy + PartialOrd + Num + ToPrimitive + Sealed,
{
    fn x(&self) -> X;
    fn y(&self) -> Y;
    fn label(&self) -> impl Into<SharedString> {
        self.x()
    }
}
