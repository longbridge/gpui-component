use std::{
    collections::VecDeque,
    io,
    net::{SocketAddr, TcpStream, ToSocketAddrs as _},
    sync::{Arc, Condvar, Mutex, OnceLock, mpsc},
    thread,
    time::{Duration, Instant},
};

const RESOLVER_WORKERS: usize = 2;
const RESOLVER_QUEUE_CAPACITY: usize = 64;

type ResolveJob = Box<dyn FnOnce() + Send + 'static>;

struct ResolverPool {
    queue: Arc<(Mutex<VecDeque<ResolveJob>>, Condvar)>,
}

impl ResolverPool {
    fn new() -> Result<Self, String> {
        let queue = Arc::new((Mutex::new(VecDeque::<ResolveJob>::new()), Condvar::new()));
        for index in 0..RESOLVER_WORKERS {
            let queue = queue.clone();
            thread::Builder::new()
                .name(format!("gpui-dns-{index}"))
                .spawn(move || resolver_worker(queue))
                .map_err(|error| format!("starting DNS resolver worker failed: {error}"))?;
        }
        Ok(Self { queue })
    }

    fn submit(&self, deadline: Instant, operation: &str, job: ResolveJob) -> Result<(), String> {
        let (queue, ready) = &*self.queue;
        let mut queue = queue
            .lock()
            .map_err(|_| "DNS resolver queue stopped unexpectedly".to_owned())?;
        while queue.len() == RESOLVER_QUEUE_CAPACITY {
            let timeout = remaining(deadline, operation)?;
            let (next, waited) = ready
                .wait_timeout(queue, timeout)
                .map_err(|_| "DNS resolver queue stopped unexpectedly".to_owned())?;
            queue = next;
            if waited.timed_out() && queue.len() == RESOLVER_QUEUE_CAPACITY {
                return Err(format!("{operation} timed out"));
            }
        }
        queue.push_back(job);
        ready.notify_one();
        Ok(())
    }
}

fn resolver_worker(queue: Arc<(Mutex<VecDeque<ResolveJob>>, Condvar)>) {
    let (queue, ready) = &*queue;
    loop {
        let job = {
            let mut queue = queue
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            while queue.is_empty() {
                queue = ready
                    .wait(queue)
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
            }
            let job = queue.pop_front().expect("the resolver queue is not empty");
            ready.notify_all();
            job
        };
        job();
    }
}

fn resolver_pool() -> Result<&'static ResolverPool, String> {
    static POOL: OnceLock<Result<ResolverPool, String>> = OnceLock::new();
    POOL.get_or_init(ResolverPool::new)
        .as_ref()
        .map_err(Clone::clone)
}

fn remaining(deadline: Instant, operation: &str) -> Result<Duration, String> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|remaining| !remaining.is_zero())
        .ok_or_else(|| format!("{operation} timed out"))
}

fn resolve_with_deadline<T, F>(deadline: Instant, operation: &str, resolve: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    let (sender, receiver) = mpsc::sync_channel(1);
    let timeout = format!("{operation} timed out");
    resolver_pool()?.submit(
        deadline,
        operation,
        Box::new(move || {
            let result = if Instant::now() >= deadline {
                Err(timeout)
            } else {
                resolve()
            };
            let _ = sender.send(result);
        }),
    )?;
    receiver
        .recv_timeout(remaining(deadline, operation)?)
        .map_err(|error| match error {
            mpsc::RecvTimeoutError::Timeout => format!("{operation} timed out"),
            mpsc::RecvTimeoutError::Disconnected => {
                format!("{operation} stopped unexpectedly")
            }
        })?
}

pub(super) fn connect_tcp(
    host: &str,
    port: u16,
    timeout: Duration,
    operation: &str,
) -> Result<(TcpStream, Instant), String> {
    let deadline = Instant::now() + timeout;
    let resolving = format!("resolving {operation}");
    let resolution_error = resolving.clone();
    let resolve_host = host.to_owned();
    let addresses: Vec<SocketAddr> = resolve_with_deadline(deadline, &resolving, move || {
        (resolve_host.as_str(), port)
            .to_socket_addrs()
            .map(|addresses| addresses.collect())
            .map_err(|error| format!("{resolution_error} failed: {error}"))
    })?;
    let stream = connect_addresses(addresses, deadline, operation, TcpStream::connect_timeout)?;
    Ok((stream, deadline))
}

fn connect_addresses<T, F>(
    addresses: Vec<SocketAddr>,
    deadline: Instant,
    operation: &str,
    mut connect: F,
) -> Result<T, String>
where
    F: FnMut(&SocketAddr, Duration) -> io::Result<T>,
{
    let mut last_error = None;
    for address in addresses {
        let timeout = remaining(deadline, &format!("connecting to {operation}"))?;
        match connect(&address, timeout) {
            Ok(stream) => return Ok(stream),
            Err(error) => last_error = Some(error),
        }
    }
    Err(format!(
        "connecting to {operation} failed: {}",
        last_error
            .map(|error| error.to_string())
            .unwrap_or_else(|| "host resolved to no addresses".to_owned())
    ))
}

pub(super) fn remaining_io_timeout(deadline: Instant, operation: &str) -> Result<Duration, String> {
    remaining(deadline, operation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc, Barrier,
        atomic::{AtomicUsize, Ordering},
    };

    const EXPECTED_RESOLVER_WORKERS: usize = 2;
    static RESOLVER_TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn dns_resolution_is_bounded_by_the_shared_deadline() {
        let _guard = RESOLVER_TEST_LOCK.lock().unwrap();
        let (release, wait) = mpsc::channel();
        let (finished, done) = mpsc::channel();
        let started = Instant::now();
        let error = resolve_with_deadline(
            started + Duration::from_millis(20),
            "resolving test host",
            move || {
                let _ = wait.recv();
                let _ = finished.send(());
                Ok(())
            },
        )
        .expect_err("deadline");

        assert_eq!(error, "resolving test host timed out");
        assert!(started.elapsed() < Duration::from_millis(250));
        release.send(()).unwrap();
        done.recv_timeout(Duration::from_millis(250)).unwrap();
    }

    #[test]
    fn dns_resolution_consumes_the_same_deadline_as_connect() {
        let _guard = RESOLVER_TEST_LOCK.lock().unwrap();
        let (release, wait) = mpsc::channel();
        let (finished, done) = mpsc::channel();
        let deadline = Instant::now() + Duration::from_millis(20);
        resolve_with_deadline(deadline, "resolving test host", move || {
            let _ = wait.recv();
            let _ = finished.send(());
            Ok(())
        })
        .expect_err("resolution must consume the deadline");

        assert_eq!(
            remaining(deadline, "connecting to test host").unwrap_err(),
            "connecting to test host timed out"
        );
        release.send(()).unwrap();
        done.recv_timeout(Duration::from_millis(250)).unwrap();
    }

    #[test]
    fn connection_tries_every_resolved_address_with_one_deadline() {
        let addresses = vec![
            "127.0.0.1:1".parse().unwrap(),
            "127.0.0.1:2".parse().unwrap(),
        ];
        let mut attempted = Vec::new();
        let connected = connect_addresses(
            addresses,
            Instant::now() + Duration::from_secs(1),
            "test host",
            |address, timeout| {
                attempted.push((*address, timeout));
                if address.port() == 1 {
                    Err(io::Error::new(io::ErrorKind::ConnectionRefused, "first"))
                } else {
                    Ok("second")
                }
            },
        )
        .expect("second address");

        assert_eq!(connected, "second");
        assert_eq!(attempted.len(), 2);
        assert!(attempted[1].1 <= attempted[0].1);
    }

    #[test]
    fn concurrent_dns_work_uses_a_small_fixed_worker_pool() {
        let _guard = RESOLVER_TEST_LOCK.lock().unwrap();
        let active = Arc::new(AtomicUsize::new(0));
        let maximum = Arc::new(AtomicUsize::new(0));
        let callers = (0..8)
            .map(|_| {
                let active = active.clone();
                let maximum = maximum.clone();
                thread::spawn(move || {
                    resolve_with_deadline(
                        Instant::now() + Duration::from_secs(2),
                        "resolving concurrent host",
                        move || {
                            let now = active.fetch_add(1, Ordering::SeqCst) + 1;
                            maximum.fetch_max(now, Ordering::SeqCst);
                            thread::sleep(Duration::from_millis(30));
                            active.fetch_sub(1, Ordering::SeqCst);
                            Ok(())
                        },
                    )
                })
            })
            .collect::<Vec<_>>();
        for caller in callers {
            caller.join().expect("caller").expect("resolution");
        }
        assert!(maximum.load(Ordering::SeqCst) <= EXPECTED_RESOLVER_WORKERS);
    }

    #[test]
    fn queued_dns_work_observes_its_own_deadline() {
        let _guard = RESOLVER_TEST_LOCK.lock().unwrap();
        let occupied = Arc::new(Barrier::new(EXPECTED_RESOLVER_WORKERS + 1));
        let release = Arc::new(Barrier::new(EXPECTED_RESOLVER_WORKERS + 1));
        let blockers = (0..EXPECTED_RESOLVER_WORKERS)
            .map(|_| {
                let occupied = occupied.clone();
                let release = release.clone();
                thread::spawn(move || {
                    resolve_with_deadline(
                        Instant::now() + Duration::from_secs(2),
                        "blocking resolver",
                        move || {
                            occupied.wait();
                            release.wait();
                            Ok(())
                        },
                    )
                })
            })
            .collect::<Vec<_>>();
        occupied.wait();

        let started = Instant::now();
        let error = resolve_with_deadline(
            started + Duration::from_millis(30),
            "queued resolver",
            || Ok(()),
        )
        .expect_err("queued work must not outlive its deadline");
        assert_eq!(error, "queued resolver timed out");
        assert!(started.elapsed() < Duration::from_millis(250));

        release.wait();
        for blocker in blockers {
            blocker.join().expect("blocker").expect("resolution");
        }
        resolve_with_deadline(
            Instant::now() + Duration::from_secs(1),
            "resolver drain sentinel",
            || Ok(()),
        )
        .expect("all earlier queued work was drained");
    }
}
