//! Deterministic shared runtime supervision.
#![cfg(feature = "transport-runtime")]
use furnace_rs_common::{core::Furnace, runtime::*};
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

#[tokio::test(start_paused = true)]
async fn shutdown_drains_then_cancels_after_30000ms() {
    struct Probe(Arc<AtomicUsize>);
    impl Drop for Probe {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    let drops = Arc::new(AtomicUsize::new(0));
    let probe = Probe(drops.clone());
    let (entered, started) = tokio::sync::oneshot::channel();
    let transport = PreparedTransport {
        name: "test",
        address: "127.0.0.1:0".into(),
        summary: String::new(),
        drain_timeout: Some(Duration::from_secs(30)),
        transition: None,
        serve: Box::new(move |listener, _, tasks| {
            Box::pin(async move {
                tasks.spawn(async move {
                    let _probe = probe;
                    entered.send(()).unwrap();
                    std::future::pending::<()>().await;
                });
                let _listener = listener;
                std::future::pending().await
            })
        }),
    };
    let application = Furnace::builder().build().await.unwrap();
    let began = tokio::time::Instant::now();
    let result = serve_prepared_transports(application, vec![transport], async move {
        started.await.unwrap();
    })
    .await;
    assert!(matches!(result, Err(ApplicationRuntimeError::DrainTimeout)));
    assert_eq!(tokio::time::Instant::now() - began, Duration::from_secs(30));
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

struct Hook {
    starts: Arc<AtomicUsize>,
    stops: Arc<AtomicUsize>,
    fail_stop: bool,
}
impl furnace_rs_common::core::LifecycleHook for Hook {
    fn name(&self) -> &str {
        "coordinator probe"
    }
    fn start<'a>(
        &'a self,
        _: &'a furnace_rs_common::core::ApplicationContext,
    ) -> furnace_rs_common::core::LifecycleFuture<'a> {
        Box::pin(async {
            self.starts.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
    }
    fn stop<'a>(
        &'a self,
        _: &'a furnace_rs_common::core::ApplicationContext,
    ) -> furnace_rs_common::core::LifecycleFuture<'a> {
        Box::pin(async {
            self.stops.fetch_add(1, Ordering::SeqCst);
            if self.fail_stop {
                Err(furnace_rs_common::core::Error::new(
                    furnace_rs_common::core::Diagnostic::new(
                        furnace_rs_common::core::FURNACE020,
                        "cleanup failed",
                        "retained cleanup cause",
                    ),
                ))
            } else {
                Ok(())
            }
        })
    }
}
fn transport(name: &'static str) -> PreparedTransport {
    PreparedTransport {
        name,
        address: "127.0.0.1:0".into(),
        summary: String::new(),
        drain_timeout: Some(Duration::from_secs(30)),
        transition: None,
        serve: Box::new(|listener, mut signal, _| {
            Box::pin(async move {
                let _listener = listener;
                if !*signal.borrow_and_update() {
                    let _ = signal.changed().await;
                }
                Ok(())
            })
        }),
    }
}
async fn app(fail_stop: bool) -> (Furnace, Arc<AtomicUsize>, Arc<AtomicUsize>) {
    let starts = Arc::new(AtomicUsize::new(0));
    let stops = Arc::new(AtomicUsize::new(0));
    let mut builder = Furnace::builder();
    builder.lifecycle_hook(Hook {
        starts: starts.clone(),
        stops: stops.clone(),
        fail_stop,
    });
    (builder.build().await.unwrap(), starts, stops)
}
#[tokio::test]
async fn second_listener_bind_failure_releases_first_and_stops_once() {
    let (app, starts, stops) = app(false).await;
    let addresses = Arc::new(std::sync::Mutex::new(Vec::new()));
    let observed = addresses.clone();
    let mut calls = 0;
    let result = serve_prepared_transports_with(
        app,
        vec![transport("http"), transport("grpc")],
        std::future::pending(),
        move |address| {
            calls += 1;
            let calls = calls;
            let observed = observed.clone();
            Box::pin(async move {
                if calls == 2 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::AddrInUse,
                        "second bind",
                    ));
                }
                let listener = tokio::net::TcpListener::bind(address).await?;
                observed.lock().unwrap().push(listener.local_addr()?);
                Ok(listener)
            })
        },
    )
    .await;
    assert!(matches!(
        result,
        Err(ApplicationRuntimeError::TransportBind {
            transport: "grpc",
            ..
        })
    ));
    assert_eq!(starts.load(Ordering::SeqCst), 1);
    assert_eq!(stops.load(Ordering::SeqCst), 1);
    let address = addresses.lock().unwrap()[0];
    assert!(tokio::net::TcpListener::bind(address).await.is_ok());
}
#[tokio::test]
async fn partial_startup_retains_operational_and_shutdown_errors() {
    let (app, _, stops) = app(true).await;
    let result = serve_prepared_transports_with(
        app,
        vec![transport("grpc")],
        std::future::pending(),
        |_| Box::pin(async { Err(std::io::Error::other("bind cause")) }),
    )
    .await;
    let Err(ApplicationRuntimeError::OperationAndShutdown {
        operation,
        shutdown,
    }) = result
    else {
        panic!("combined failure expected")
    };
    assert!(operation.to_string().contains("bind cause"));
    assert!(
        std::error::Error::source(&shutdown)
            .unwrap()
            .to_string()
            .contains("cleanup failed")
    );
    assert_eq!(stops.load(Ordering::SeqCst), 1);
}
#[tokio::test]
async fn first_server_failure_stops_sibling() {
    let (app, _, stops) = app(false).await;
    let mut first = transport("grpc");
    first.serve = Box::new(|_, _, _| {
        Box::pin(async {
            Err(ApplicationRuntimeError::TransportServe {
                transport: "grpc",
                source: Box::new(std::io::Error::other("serve cause")),
            })
        })
    });
    let sibling_drops = Arc::new(AtomicUsize::new(0));
    let observed = sibling_drops.clone();
    let mut sibling = transport("http");
    sibling.serve = Box::new(move |listener, mut signal, _| {
        Box::pin(async move {
            let _listener = listener;
            if !*signal.borrow_and_update() {
                let _ = signal.changed().await;
            }
            observed.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
    });
    let result = serve_prepared_transports(app, vec![first, sibling], std::future::pending()).await;
    assert!(matches!(
        result,
        Err(ApplicationRuntimeError::TransportServe {
            transport: "grpc",
            ..
        })
    ));
    assert_eq!(sibling_drops.load(Ordering::SeqCst), 1);
    assert_eq!(stops.load(Ordering::SeqCst), 1);
}
#[tokio::test]
async fn health_changes_to_not_serving_before_drain() {
    let (app, _, _) = app(false).await;
    let status = Arc::new(AtomicUsize::new(0));
    let health = status.clone();
    let mut server = transport("grpc");
    server.transition = Some(Box::new(move |serving| {
        let health = health.clone();
        Box::pin(async move {
            health.store(if serving { 1 } else { 2 }, Ordering::SeqCst);
        })
    }));
    let (entered, admitted) = tokio::sync::oneshot::channel();
    let health = status.clone();
    server.serve = Box::new(move |listener, mut signal, _| {
        Box::pin(async move {
            let _listener = listener;
            assert_eq!(health.load(Ordering::SeqCst), 1);
            entered.send(()).unwrap();
            signal.changed().await.unwrap();
            assert_eq!(health.load(Ordering::SeqCst), 2);
            Ok(())
        })
    });
    serve_prepared_transports(app, vec![server], async {
        admitted.await.unwrap();
    })
    .await
    .unwrap();
}
