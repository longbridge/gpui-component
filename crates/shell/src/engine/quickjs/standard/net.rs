use std::{
    io::{Read as _, Write as _},
    net::{Shutdown, TcpStream, ToSocketAddrs as _},
    sync::{Arc, Mutex},
    time::Duration,
};

use rquickjs::{
    Ctx, Exception, IntoJs, Object, Promise, Result, Value,
    function::{Func, Opt},
    module::{Declarations, Exports, ModuleDef},
};

use super::super::{host, scheduler};

const IO_LIMIT: usize = 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(30);

pub(super) struct NetModule;

impl ModuleDef for NetModule {
    fn declare(declarations: &Declarations) -> Result<()> {
        declarations.declare("connect")?;
        declarations.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        let net = Object::new(ctx.clone())?;
        net.set("connect", Func::from(connect))?;
        exports.export("connect", net.get::<_, Value>("connect")?)?;
        exports.export("default", net.into_value())?;
        Ok(())
    }
}

fn connect<'js>(ctx: Ctx<'js>, host_name: String, port: u16) -> Result<Promise<'js>> {
    let normalized = host_name.to_ascii_lowercase();
    if !host::capabilities().may_reach(&normalized) {
        return Err(Exception::throw_type(
            &ctx,
            &format!(
                "network access to `{normalized}` is not granted; add it to capabilities.network.hosts"
            ),
        ));
    }
    scheduler::blocking(&ctx, "net.connect(host, port)", move || {
        let address = (host_name.as_str(), port)
            .to_socket_addrs()
            .map_err(|error| format!("resolving {host_name}:{port} failed: {error}"))?
            .next()
            .ok_or_else(|| format!("{host_name}:{port} resolved to no address"))?;
        let stream = TcpStream::connect_timeout(&address, TIMEOUT)
            .map_err(|error| format!("connecting to {host_name}:{port} failed: {error}"))?;
        stream.set_read_timeout(Some(TIMEOUT)).ok();
        stream.set_write_timeout(Some(TIMEOUT)).ok();
        Ok(Socket(Arc::new(Mutex::new(stream))))
    })
}

struct Socket(Arc<Mutex<TcpStream>>);

impl<'js> IntoJs<'js> for Socket {
    fn into_js(self, ctx: &Ctx<'js>) -> Result<Value<'js>> {
        let socket = Object::new(ctx.clone())?;
        let writer = self.0.clone();
        socket.set(
            "write",
            Func::from(move |ctx: Ctx<'js>, data: String| -> Result<Promise<'js>> {
                if data.len() > IO_LIMIT {
                    return Err(Exception::throw_range(
                        &ctx,
                        &format!("socket write exceeded the {IO_LIMIT} byte limit"),
                    ));
                }
                let stream = writer.clone();
                scheduler::blocking(&ctx, "socket.write(data)", move || {
                    stream
                        .lock()
                        .map_err(|_| "socket lock was poisoned".to_owned())?
                        .write_all(data.as_bytes())
                        .map_err(|error| format!("socket write failed: {error}"))
                })
            }),
        )?;
        let reader = self.0.clone();
        socket.set(
            "read",
            Func::from(
                move |ctx: Ctx<'js>, size: Opt<usize>| -> Result<Promise<'js>> {
                    let size = size.0.unwrap_or(64 * 1024);
                    if size > IO_LIMIT {
                        return Err(Exception::throw_range(
                            &ctx,
                            &format!("socket read exceeded the {IO_LIMIT} byte limit"),
                        ));
                    }
                    let stream = reader.clone();
                    scheduler::blocking(&ctx, "socket.read(size)", move || {
                        let mut bytes = vec![0; size];
                        let count = stream
                            .lock()
                            .map_err(|_| "socket lock was poisoned".to_owned())?
                            .read(&mut bytes)
                            .map_err(|error| format!("socket read failed: {error}"))?;
                        bytes.truncate(count);
                        Ok(String::from_utf8_lossy(&bytes).into_owned())
                    })
                },
            ),
        )?;
        let closer = self.0;
        socket.set(
            "close",
            Func::from(move || {
                let _ = closer.lock().map(|stream| stream.shutdown(Shutdown::Both));
            }),
        )?;
        Ok(socket.into_value())
    }
}
