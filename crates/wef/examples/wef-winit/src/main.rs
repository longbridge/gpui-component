use std::{rc::Rc, time::Duration};

use image::{GenericImage, RgbaImage, buffer::ConvertBuffer};
use softbuffer::Surface;
use wef::{
    Browser, BrowserHandler, DirtyRects, ImageBuffer, KeyCode, KeyModifier, LogicalUnit,
    MouseButton, PaintElementType, Rect, Settings,
};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{Ime, Modifiers, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    keyboard::{ModifiersState, NamedKey},
    window::{Window, WindowId},
};

type BoxError = Box<dyn std::error::Error>;

enum UserEvent {
    WefMessagePump,
    Exit,
}

struct State {
    scale_factor: f32,
    browser: Browser,
}

impl State {
    fn new(event_loop_proxy: EventLoopProxy<UserEvent>, window: Window) -> Result<Self, BoxError> {
        let window = Rc::new(window);
        let inner_size = window.inner_size();
        let context = softbuffer::Context::new(window.clone())?;
        let mut surface = Surface::new(&context, window.clone())?;

        surface.resize(
            inner_size.width.try_into().expect("valid surface width"),
            inner_size.height.try_into().expect("valid surface height"),
        )?;

        let scale_factor = window.scale_factor() as f32;

        // Create the browser instance
        let browser = Browser::builder()
            .size(inner_size.width, inner_size.height)
            .device_scale_factor(scale_factor)
            .url("https://www.google.com")
            .handler(MyHandler {
                scale_factor,
                surface,
                window,
                view_image: None,
                popup_rect: Rect::default(),
                popup_image: None,
                event_loop_proxy,
            })
            .build();
        browser.set_focus(true);

        Ok(Self {
            scale_factor,
            browser,
        })
    }
}

struct App {
    event_loop_proxy: EventLoopProxy<UserEvent>,
    state: Option<State>,
    key_modifiers: KeyModifier,
}

impl App {
    fn new(event_loop_proxy: EventLoopProxy<UserEvent>) -> Self {
        Self {
            event_loop_proxy,
            state: None,
            key_modifiers: KeyModifier::empty(),
        }
    }

    #[inline]
    fn browser(&self) -> Option<&Browser> {
        self.state.as_ref().map(|state| &state.browser)
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = event_loop
            .create_window(
                Window::default_attributes().with_inner_size(LogicalSize::new(1024, 768)),
            )
            .unwrap();
        window.set_ime_allowed(true);

        self.state =
            Some(State::new(self.event_loop_proxy.clone(), window).expect("create window"));
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::WefMessagePump => wef::do_message_work(),
            UserEvent::Exit => event_loop.exit(),
        }
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                if let Some(browser) = self.browser() {
                    browser.close();
                }
            }
            WindowEvent::Resized(size) => {
                if let Some(state) = &mut self.state {
                    state.browser.resize(wef::Size::new(
                        wef::PhysicalUnit(size.width as i32),
                        wef::PhysicalUnit(size.height as i32),
                    ));
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let scale_factor = self.state.as_ref().unwrap().scale_factor;
                let position = position.to_logical::<f32>(scale_factor as f64);
                if let Some(browser) = self.browser() {
                    browser.send_mouse_move_event(
                        wef::Point::new(
                            wef::LogicalUnit(position.x as i32),
                            wef::LogicalUnit(position.y as i32),
                        ),
                        self.key_modifiers,
                    );
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let button = match button {
                    winit::event::MouseButton::Left => MouseButton::Left,
                    winit::event::MouseButton::Middle => MouseButton::Middle,
                    winit::event::MouseButton::Right => MouseButton::Right,
                    _ => return,
                };
                let pressed = state.is_pressed();
                if let Some(browser) = self.browser() {
                    browser.send_mouse_click_event(button, !pressed, 1, self.key_modifiers);
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let (delta_x, delta_y) = match delta {
                    MouseScrollDelta::LineDelta(x, y) => (50 * x as i32, 50 * y as i32),
                    MouseScrollDelta::PixelDelta(delta) => (delta.x as _, delta.y as _),
                };
                if let Some(browser) = self.browser() {
                    browser.send_mouse_wheel_event(wef::Point::new(
                        wef::LogicalUnit(delta_x),
                        wef::LogicalUnit(delta_y),
                    ));
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.key_modifiers = convert_key_modifiers(modifiers);
            }
            WindowEvent::KeyboardInput { event, .. } => match event.logical_key.as_ref() {
                winit::keyboard::Key::Named(named_key) => {
                    if let Some(key_code) = convert_key_code(named_key) {
                        if let Some(browser) = self.browser() {
                            browser.send_key_event(
                                event.state.is_pressed(),
                                key_code,
                                self.key_modifiers,
                            );
                        }
                    }
                }
                winit::keyboard::Key::Character(s) if event.state.is_pressed() => {
                    if let Some(browser) = self.browser() {
                        for ch in s.chars() {
                            browser.send_char_event(ch as u16);
                        }
                    }
                }
                _ => {}
            },
            WindowEvent::Ime(ime) => match ime {
                Ime::Preedit(text, range) => {
                    let (start, end) = range.unwrap_or_default();
                    if let Some(browser) = self.browser() {
                        browser.ime_set_composition(&text, start, end);
                    }
                }
                Ime::Commit(text) => {
                    if let Some(browser) = self.browser() {
                        browser.ime_commit(&text)
                    }
                }
                _ => {}
            },
            WindowEvent::Focused(focused) => {
                if let Some(browser) = self.browser() {
                    browser.set_focus(focused);
                }
            }
            _ => (),
        }
    }
}

struct MyHandler {
    scale_factor: f32,
    surface: Surface<Rc<Window>, Rc<Window>>,
    window: Rc<Window>,
    view_image: Option<RgbaImage>,
    popup_rect: Rect<LogicalUnit<i32>>,
    popup_image: Option<RgbaImage>,
    event_loop_proxy: EventLoopProxy<UserEvent>,
}

impl BrowserHandler for MyHandler {
    fn on_closed(&mut self) {
        _ = self.event_loop_proxy.send_event(UserEvent::Exit);
    }

    fn on_paint(
        &mut self,
        type_: PaintElementType,
        _dirty_rects: &DirtyRects,
        image_buffer: ImageBuffer,
    ) {
        match type_ {
            PaintElementType::View => match &mut self.view_image {
                Some(view_image)
                    if view_image.width() == image_buffer.width()
                        && view_image.height() == image_buffer.height() =>
                {
                    view_image.copy_from_slice(&image_buffer);
                }
                _ => self.view_image = Some(image_buffer.convert()),
            },
            PaintElementType::Popup => {
                self.popup_image = Some(image_buffer.convert());
            }
        }

        if let Some(view_image) = &self.view_image {
            self.surface
                .resize(
                    view_image.width().try_into().expect("valid surface width"),
                    view_image
                        .height()
                        .try_into()
                        .expect("valid surface height"),
                )
                .expect("resize surface");
            let mut buffer = self.surface.buffer_mut().unwrap();

            let mut dest = image::ImageBuffer::<image::Rgba<u8>, &mut [u8]>::from_raw(
                view_image.width(),
                view_image.height(),
                unsafe {
                    std::slice::from_raw_parts_mut(
                        buffer.as_mut_ptr() as *mut u8,
                        (view_image.width() * view_image.height() * 4) as usize,
                    )
                },
            )
            .unwrap();
            dest.copy_from_slice(view_image);

            if let Some(popup_image) = &self.popup_image {
                let origin = self
                    .popup_rect
                    .origin()
                    .map(|x| x.to_physical(self.scale_factor));
                dest.copy_from(popup_image, origin.x.0 as u32, origin.y.0 as u32)
                    .unwrap();
            }

            buffer.present().unwrap();
        }
    }

    fn on_title_changed(&mut self, title: &str) {
        self.window.set_title(title);
    }

    fn on_popup_show(&mut self, show: bool) {
        if !show {
            self.popup_image = None;
        }
    }

    fn on_popup_position(&mut self, rect: Rect<LogicalUnit<i32>>) {
        self.popup_rect = rect;
    }
}

fn convert_key_code(key: NamedKey) -> Option<KeyCode> {
    match key {
        NamedKey::Backspace => Some(KeyCode::Backspace),
        NamedKey::Delete => Some(KeyCode::Delete),
        NamedKey::Tab => Some(KeyCode::Tab),
        NamedKey::Enter => Some(KeyCode::Enter),
        NamedKey::PageUp => Some(KeyCode::PageUp),
        NamedKey::PageDown => Some(KeyCode::PageDown),
        NamedKey::End => Some(KeyCode::End),
        NamedKey::Home => Some(KeyCode::Home),
        NamedKey::ArrowLeft => Some(KeyCode::ArrowLeft),
        NamedKey::ArrowUp => Some(KeyCode::ArrowUp),
        NamedKey::ArrowRight => Some(KeyCode::ArrowRight),
        NamedKey::ArrowDown => Some(KeyCode::ArrowDown),
        _ => None,
    }
}

fn convert_key_modifiers(modifiers: Modifiers) -> KeyModifier {
    let mut key_modifiers = KeyModifier::empty();
    if modifiers.state().contains(ModifiersState::SHIFT) {
        key_modifiers |= KeyModifier::SHIFT;
    }
    if modifiers.state().contains(ModifiersState::CONTROL) {
        key_modifiers |= KeyModifier::CONTROL;
    }
    if modifiers.state().contains(ModifiersState::ALT) {
        key_modifiers |= KeyModifier::ALT;
    }
    key_modifiers
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::<UserEvent>::with_user_event().build()?;
    let event_loop_proxy = event_loop.create_proxy();

    if cfg!(target_os = "linux") {
        std::thread::spawn({
            let event_loop_proxy = event_loop_proxy.clone();
            move || {
                loop {
                    std::thread::sleep(Duration::from_millis(1000 / 60));
                    if event_loop_proxy
                        .send_event(UserEvent::WefMessagePump)
                        .is_err()
                    {
                        break;
                    }
                }
            }
        });
    }

    let mut app = App::new(event_loop_proxy);
    event_loop.run_app(&mut app)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    wef::launch(Settings::new(), run)?;
    Ok(())
}
