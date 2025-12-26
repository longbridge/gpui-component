use gpui::{AnyView, App, AppContext as _, Entity, Hsla, Pixels, Render, Window, px};
use gpui_component::dock::PanelControl;

mod accordion_story;
mod alert_story;
mod avatar_story;
mod badge_story;
mod button_story;
mod calendar_story;
mod chart_story;
mod checkbox_story;
mod clipboard_story;
mod collapsible_story;
mod color_picker_story;
mod date_picker_story;
mod description_list_story;
mod dialog_story;
mod divider_story;
mod dropdown_button_story;
mod form_story;
mod group_box_story;
mod icon_story;
mod image_story;
mod input_story;
mod kbd_story;
mod label_story;
mod list_story;
mod menu_story;
mod notification_story;
mod number_input_story;
mod otp_input_story;
mod pagination_story;
mod popover_story;
mod progress_story;
mod radio_story;
mod resizable_story;
mod scrollbar_story;
mod select_story;
mod settings_story;
mod sheet_story;
mod sidebar_story;
mod skeleton_story;
mod slider_story;
mod spinner_story;
mod stepper_story;
mod switch_story;
mod table_story;
mod tabs_story;
mod tag_story;
mod textarea_story;
mod toggle_story;
mod tooltip_story;
mod tree_story;
mod virtual_list_story;
mod welcome_story;

pub use accordion_story::AccordionStory;
pub use alert_story::AlertStory;
pub use avatar_story::AvatarStory;
pub use badge_story::BadgeStory;
pub use button_story::ButtonStory;
pub use calendar_story::CalendarStory;
pub use chart_story::ChartStory;
pub use checkbox_story::CheckboxStory;
pub use clipboard_story::ClipboardStory;
pub use collapsible_story::CollapsibleStory;
pub use color_picker_story::ColorPickerStory;
pub use date_picker_story::DatePickerStory;
pub use description_list_story::DescriptionListStory;
pub use dialog_story::DialogStory;
pub use divider_story::DividerStory;
pub use dropdown_button_story::DropdownButtonStory;
pub use form_story::FormStory;
pub use group_box_story::GroupBoxStory;
pub use icon_story::IconStory;
pub use image_story::ImageStory;
pub use input_story::InputStory;
pub use kbd_story::KbdStory;
pub use label_story::LabelStory;
pub use list_story::ListStory;
pub use menu_story::MenuStory;
pub use notification_story::NotificationStory;
pub use number_input_story::NumberInputStory;
pub use otp_input_story::OtpInputStory;
pub use pagination_story::PaginationStory;
pub use popover_story::PopoverStory;
pub use progress_story::ProgressStory;
pub use radio_story::RadioStory;
pub use resizable_story::ResizableStory;
pub use scrollbar_story::ScrollbarStory;
pub use select_story::SelectStory;
pub use settings_story::SettingsStory;
pub use sheet_story::SheetStory;
pub use sidebar_story::SidebarStory;
pub use skeleton_story::SkeletonStory;
pub use slider_story::SliderStory;
pub use spinner_story::SpinnerStory;
pub use stepper_story::StepperStory;
pub use switch_story::SwitchStory;
pub use table_story::TableStory;
pub use tabs_story::TabsStory;
pub use tag_story::TagStory;
pub use textarea_story::TextareaStory;
pub use toggle_story::ToggleStory;
pub use tooltip_story::TooltipStory;
pub use tree_story::TreeStory;
pub use virtual_list_story::VirtualListStory;

pub use welcome_story::WelcomeStory;

pub(crate) fn init(cx: &mut App) {
    input_story::init(cx);
    number_input_story::init(cx);
    textarea_story::init(cx);
    select_story::init(cx);
    popover_story::init(cx);
    menu_story::init(cx);
    tooltip_story::init(cx);
    otp_input_story::init(cx);
    tree_story::init(cx);
}

pub trait Story: Render + Sized {
    fn klass() -> &'static str {
        std::any::type_name::<Self>().split("::").last().unwrap()
    }

    fn title() -> &'static str;

    fn description() -> &'static str {
        ""
    }

    fn closable() -> bool {
        true
    }

    fn zoomable() -> Option<PanelControl> {
        Some(PanelControl::default())
    }

    fn title_bg() -> Option<Hsla> {
        None
    }

    fn paddings() -> Pixels {
        px(16.)
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render>;

    fn on_active(&mut self, active: bool, window: &mut Window, cx: &mut App) {
        let _ = active;
        let _ = window;
        let _ = cx;
    }

    fn on_active_any(view: AnyView, active: bool, window: &mut Window, cx: &mut App)
    where
        Self: 'static,
    {
        if let Some(story) = view.downcast::<Self>().ok() {
            cx.update_entity(&story, |story, cx| {
                story.on_active(active, window, cx);
            });
        }
    }
}
