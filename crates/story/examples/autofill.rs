use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::{
    ActiveTheme as _, Root,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputContentType, InputState},
    label::Label,
    v_flex,
};
use gpui_component_assets::Assets;

pub struct Example {
    username: Entity<InputState>,
    password: Entity<InputState>,
    new_password: Entity<InputState>,
    one_time_code: Entity<InputState>,
}

impl Example {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            username: cx.new(|cx| InputState::new(window, cx).placeholder("Username")),
            password: cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("Password")
                    .masked(true)
            }),
            new_password: cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("New password")
                    .masked(true)
            }),
            one_time_code: cx.new(|cx| InputState::new(window, cx).placeholder("123456")),
        }
    }

    fn field(
        label: &'static str,
        input: impl IntoElement,
        action: Option<impl IntoElement>,
    ) -> impl IntoElement {
        h_flex()
            .w_full()
            .items_center()
            .gap_3()
            .child(Label::new(label).w_32().flex_shrink_0())
            .child(input)
            .when_some(action, |this, action| this.child(action))
    }
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .id("autofill-example")
            .size_full()
            .items_center()
            .justify_center()
            .bg(cx.theme().background)
            .child(
                v_flex()
                    .w(px(520.))
                    .max_w_full()
                    .gap_4()
                    .p_8()
                    .child(Self::field(
                        "Username",
                        Input::new(&self.username)
                            .content_type(InputContentType::Username)
                            .flex_1(),
                        None::<AnyElement>,
                    ))
                    .child(Self::field(
                        "Password",
                        Input::new(&self.password)
                            .content_type(InputContentType::Password)
                            .mask_toggle()
                            .flex_1(),
                        Some(
                            Button::new("sign-in")
                                .primary()
                                .label("Sign in")
                                .into_any_element(),
                        ),
                    ))
                    .child(Self::field(
                        "New password",
                        Input::new(&self.new_password)
                            .content_type(InputContentType::NewPassword)
                            .mask_toggle()
                            .flex_1(),
                        None::<AnyElement>,
                    ))
                    .child(Self::field(
                        "Code",
                        Input::new(&self.one_time_code)
                            .content_type(InputContentType::OneTimeCode)
                            .flex_1(),
                        None::<AnyElement>,
                    )),
            )
    }
}

fn main() {
    let app = gpui_platform::application().with_assets(Assets);

    app.run(move |cx| {
        gpui_component::init(cx);
        cx.activate(true);

        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(720.), px(520.)), cx)),
            titlebar: Some(TitlebarOptions {
                title: Some("AutoFill".into()),
                ..Default::default()
            }),
            ..Default::default()
        };

        cx.spawn(async move |cx| {
            cx.open_window(window_options, |window, cx| {
                let view = cx.new(|cx| Example::new(window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("failed to open window");
        })
        .detach();
    });
}
