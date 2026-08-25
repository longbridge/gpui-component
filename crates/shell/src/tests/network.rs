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

const NET_LIMIT_PROBE: &str = r#"
import { View, v_flex, text, spawn, with_cx } from "gpui";
import { connect } from "net";

export default class Probe extends View {
  init() {
    this.state = "pending";
    spawn(async () => {
      const socket = await connect("127.0.0.1", __PORT__);
      const errors = [];
      try { await socket.write("x".repeat(1048577)); }
      catch (error) { errors.push(error.message); }
      try { await socket.read(1048577); }
      catch (error) { errors.push(error.message); }
      socket.close();
      this.state = errors.join("|");
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

#[gpui::test]
fn net_connect_is_denied_by_default(cx: &mut TestAppContext) {
    cx.update(crate::init);
    crate::set_capabilities(Capabilities::new());
    let source = NET_PROBE.replace("__PORT__", "9");
    let runtime = ShellRuntime::new().expect("runtime");
    cx.update(|cx| runtime.set_global(cx));
    let view_type = runtime.load_source("denied-net.js", &source).expect("load");
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

#[gpui::test]
fn fetch_reauthorizes_every_redirect_target(cx: &mut TestAppContext) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("HTTP listener");
    let address = listener.local_addr().expect("listener address");
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("HTTP connection");
        let mut request = [0; 1024];
        let _ = stream.read(&mut request);
        let response = format!(
            "HTTP/1.1 302 Found\r\nLocation: http://localhost:{}/denied\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            address.port()
        );
        stream.write_all(response.as_bytes()).expect("redirect");
    });

    let source = FETCH_PROBE.replace("__URL__", &format!("http://{address}/redirect"));
    let (_runtime, view, mut context) = probe(cx, &source);
    context.run_until_parked();
    draw(&mut context, &view);
    let rendered = snapshot(&mut context, &view);
    assert!(
        rendered.contains("rejected:")
            && rendered.contains("redirect target")
            && rendered.contains("localhost"),
        "{rendered}"
    );
    server.join().expect("HTTP server");
}

#[gpui::test]
fn fetch_rejects_a_response_over_the_buffer_limit(cx: &mut TestAppContext) {
    const TOO_LARGE: usize = 8 * 1024 * 1024 + 1;
    let listener = TcpListener::bind("127.0.0.1:0").expect("HTTP listener");
    let address = listener.local_addr().expect("listener address");
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("HTTP connection");
        let mut request = [0; 1024];
        let _ = stream.read(&mut request);
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {TOO_LARGE}\r\nConnection: close\r\n\r\n"
        )
        .expect("headers");
        let body = vec![b'x'; TOO_LARGE];
        stream.write_all(&body).expect("oversized body");
    });

    let source = FETCH_PROBE.replace("__URL__", &format!("http://{address}/large"));
    let (_runtime, view, mut context) = probe(cx, &source);
    context.run_until_parked();
    draw(&mut context, &view);
    let rendered = snapshot(&mut context, &view);
    assert!(
        rendered.contains("rejected:") && rendered.contains("8388608 byte limit"),
        "{rendered}"
    );
    server.join().expect("HTTP server");
}

#[gpui::test]
fn net_rejects_read_and_write_calls_over_the_per_call_limit(cx: &mut TestAppContext) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("TCP listener");
    let port = listener.local_addr().expect("listener address").port();
    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().expect("TCP connection");
        stream
            .set_read_timeout(Some(std::time::Duration::from_millis(250)))
            .expect("timeout");
        let mut stream = stream;
        let mut byte = [0];
        let _ = stream.read(&mut byte);
    });

    let source = NET_LIMIT_PROBE.replace("__PORT__", &port.to_string());
    let (_runtime, view, mut context) = probe(cx, &source);
    context.run_until_parked();
    draw(&mut context, &view);
    let rendered = snapshot(&mut context, &view);
    assert!(
        rendered.contains("socket write exceeded the 1048576 byte limit")
            && rendered.contains("socket read exceeded the 1048576 byte limit"),
        "{rendered}"
    );
    server.join().expect("TCP server");
}

#[gpui::test]
fn two_runtimes_keep_distinct_network_policies(cx: &mut TestAppContext) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("HTTP listener");
    let address = listener.local_addr().expect("listener address");
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("one allowed connection");
        let mut request = [0; 1024];
        let _ = stream.read(&mut request);
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nhello")
            .expect("HTTP response");
    });
    let source = FETCH_PROBE.replace("__URL__", &format!("http://{address}/policy"));

    cx.update(crate::init);
    crate::set_capabilities(Capabilities::new().network_hosts(["127.0.0.1".to_owned()]));
    let allowed_runtime = ShellRuntime::new().expect("allowed runtime");
    cx.update(|cx| allowed_runtime.set_global(cx));
    let allowed_type = allowed_runtime
        .load_source("allowed-network.js", &source)
        .expect("allowed source");
    let allowed_window = cx.add_window(|_, _| Empty);
    let mut allowed_context = VisualTestContext::from_window(*allowed_window.deref(), cx);
    let allowed_view = allowed_context
        .update(|window, cx| allowed_runtime.instantiate_view(&allowed_type, window, cx))
        .expect("allowed view");

    crate::set_capabilities(Capabilities::new());
    let denied_runtime = ShellRuntime::new().expect("denied runtime");
    cx.update(|cx| denied_runtime.set_global(cx));
    let denied_type = denied_runtime
        .load_source("denied-network.js", &source)
        .expect("denied source");
    let denied_window = cx.add_window(|_, _| Empty);
    let mut denied_context = VisualTestContext::from_window(*denied_window.deref(), cx);
    let denied_view = denied_context
        .update(|window, cx| denied_runtime.instantiate_view(&denied_type, window, cx))
        .expect("denied view");

    allowed_context.run_until_parked();
    denied_context.run_until_parked();
    draw(&mut allowed_context, &allowed_view);
    draw(&mut denied_context, &denied_view);
    assert!(
        snapshot(&mut allowed_context, &allowed_view).contains("200|true|hello"),
        "allowed runtime lost its captured policy"
    );
    let denied = snapshot(&mut denied_context, &denied_view);
    assert!(
        denied.contains("rejected:") && denied.contains("capabilities.network.hosts"),
        "{denied}"
    );
    server.join().expect("HTTP server");
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
