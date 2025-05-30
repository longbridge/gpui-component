// filepath: todo-app-gpui/todo-app-gpui/src/views/provider_form.rs
use gpui::*;
use crate::models::provider_config::ProviderConfig;

pub struct ProviderForm {
    config: ProviderConfig,
    name_input: Entity<TextInput>,
    url_input: Entity<TextInput>,
    submit_button: Entity<Button>,
}

impl ProviderForm {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let name_input = cx.new(|cx| TextInput::new(cx).placeholder("Provider Name"));
        let url_input = cx.new(|cx| TextInput::new(cx).placeholder("Provider URL"));
        let submit_button = cx.new(|cx| Button::new(cx).label("Submit").on_click(|_, cx| {
            // Handle form submission logic here
        }));

        Self {
            config: ProviderConfig::default(),
            name_input,
            url_input,
            submit_button,
        }
    }

    pub fn view(&mut self, cx: &mut Context<Self>) -> Entity<Self> {
        cx.new(|cx| {
            div()
                .child(self.name_input.clone())
                .child(self.url_input.clone())
                .child(self.submit_button.clone())
        })
    }
}

impl Render for ProviderForm {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.view(cx)
    }
}