use std::{
    io::{Read as _, Write as _},
    net::TcpListener,
    ops::Deref,
    thread,
};

use gpui::{Entity, IntoElement as _, TestAppContext, VisualTestContext};

use crate::{Capabilities, ScriptView, ShellRuntime};

const FETCH_PROBE: &str = r#"
import { View, v_flex, text, spawn, with_cx } from "gpui";

export default class Probe extends View {
  init() {
    this.state = "pending";
    spawn(async () => {
      try {
        const response = await fetch("__URL__");
        this.state = `${response.status}|${response.ok}|${await response.text()}`;
      } catch (error) {
        this.state = `rejected:${error.message}`;
      }
      with_cx((cx) => cx.notify());
    });
  }
  render() { return v_flex().child(text(this.state)); }
}
"#;

const NET_PROBE: &str = r#"
import { View, v_flex, text, spawn, with_cx } from "gpui";
import { connect } from "net";

export default class Probe extends View {
  init() {
    this.state = "pending";
    spawn(async () => {
      try {
        const socket = await connect("127.0.0.1", __PORT__);
        await socket.write("ping");
        this.state = await socket.read(4);
        socket.close();
      } catch (error) {
        this.state = `rejected:${error.message}`;
      }
      with_cx((cx) => cx.notify());
    });
  }
  render() { return v_flex().child(text(this.state)); }
}
"#;

#[gpui::test]
fn fetch_runs_off_thread_and_obeys_the_active_policy(cx: &mut TestAppContext) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("HTTP listener");
    let address = listener.local_addr().expect("listener address");
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("HTTP connection");
        let mut request = [0; 1024];
        let _ = stream.read(&mut request);
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nhello")
            .expect("HTTP response");
    });

    let source = FETCH_PROBE.replace("__URL__", &format!("http://{address}/probe"));
    let (_runtime, view, mut context) = probe(cx, &source);
    draw(&mut context, &view);
    assert!(snapshot(&mut context, &view).contains("pending"));
    context.run_until_parked();
    draw(&mut context, &view);
    assert!(
        snapshot(&mut context, &view).contains("200|true|hello"),
        "fetch did not settle through the script boundary"
    );
    server.join().expect("HTTP server");
}

#[gpui::test]
fn net_connect_is_bounded_and_capability_gated(cx: &mut TestAppContext) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("TCP listener");
    let port = listener.local_addr().expect("listener address").port();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("TCP connection");
        let mut request = [0; 4];
        stream.read_exact(&mut request).expect("read ping");
        assert_eq!(&request, b"ping");
        stream.write_all(b"pong").expect("write pong");
    });

    let source = NET_PROBE.replace("__PORT__", &port.to_string());
    let (_runtime, view, mut context) = probe(cx, &source);
    context.run_until_parked();
    draw(&mut context, &view);
    assert!(snapshot(&mut context, &view).contains("pong"));
    server.join().expect("TCP server");
}

#[gpui::test]
fn network_is_denied_by_default(cx: &mut TestAppContext) {
    cx.update(crate::init);
    crate::set_capabilities(Capabilities::new());
    let runtime = ShellRuntime::new().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let source = FETCH_PROBE.replace("__URL__", "http://127.0.0.1:9/");
    let view_type = runtime
        .load_source("denied-fetch.js", &source)
        .expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("the async task catches the denial");
    context.run_until_parked();
    draw(&mut context, &view);
    let rendered = snapshot(&mut context, &view);
    assert!(
        rendered.contains("rejected:") && rendered.contains("capabilities.network.hosts"),
        "{rendered}"
    );
}

fn probe(
    cx: &mut TestAppContext,
    source: &str,
) -> (
    std::rc::Rc<ShellRuntime>,
    Entity<ScriptView>,
    VisualTestContext,
) {
    cx.update(crate::init);
    crate::set_capabilities(Capabilities::new().network_hosts(["127.0.0.1".to_owned()]));
    let runtime = ShellRuntime::new().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let view_type = runtime.load_source("network.js", source).expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate");
    (runtime, view, context)
}

fn draw(context: &mut VisualTestContext, view: &Entity<ScriptView>) {
    let view = view.clone();
    context.draw(
        gpui::Point::default(),
        gpui::size(gpui::px(400.), gpui::px(300.)),
        move |_, _| view.into_any_element(),
    );
}

fn snapshot(context: &mut VisualTestContext, view: &Entity<ScriptView>) -> String {
    context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    })
}

struct Empty;

impl gpui::Render for Empty {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        gpui::div()
    }
}
