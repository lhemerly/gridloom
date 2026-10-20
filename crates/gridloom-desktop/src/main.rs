use clap::Parser;
use gridloom_server::benchmark::run_performance_benchmarks;
use gridloom_server::start_server;
use std::env;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(
    name = "gridloom",
    about = "Gridloom - Local-first SQL Worksheet Desktop App"
)]
struct Args {
    #[arg(short, long, default_value_t = 0)]
    port: u16,

    #[arg(long, default_value_t = false)]
    bench: bool,

    #[arg(long, default_value_t = false)]
    server: bool,

    #[arg(long, default_value_t = false)]
    no_samples: bool,
}

#[allow(unreachable_code)]
fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args = Args::parse();

    if args.bench {
        println!("Running Gridloom V1 Performance Acceptance Protocol Benchmarks...\n");
        let report = run_performance_benchmarks();
        println!("============================================================");
        println!(" GRIDLOOM V1 PERFORMANCE ACCEPTANCE REPORT");
        println!("============================================================");
        for r in &report.results {
            println!("\n* Workload: {}", r.workload_name);
            println!("  Operations:  {}", r.operations);
            println!("  Elapsed:     {:.2} ms", r.elapsed_ms);
            println!("  Throughput:  {:.1} ops/sec", r.throughput_ops_per_sec);
            println!(
                "  Peak RSS:    {:?} KB (process high-water mark)",
                r.memory_peak_rss_kb
            );
            println!("  Status:      {}", r.status);
            println!("  Notes:       {}", r.notes);
        }
        println!("\n------------------------------------------------------------");
        println!(
            "Total workloads: {} in {:.2} ms",
            report.total_workloads, report.total_elapsed_ms
        );
        println!(
            "Correctness Gate: {}",
            if report.passed_correctness_gate {
                "PASSED"
            } else {
                "FAILED"
            }
        );
        println!(
            "Resource Gate:    {}",
            match report.passed_resource_gate {
                Some(true) => "PASSED",
                Some(false) => "FAILED",
                None => "UNMEASURED",
            }
        );
        println!("============================================================\n");
        return report.ensure_passed();
    }

    let rt = tokio::runtime::Runtime::new()?;
    let (actual_port, server_handle) =
        rt.block_on(async { start_server(args.port, !args.no_samples).await })?;

    println!(
        "Gridloom Core Engine active at http://127.0.0.1:{}",
        actual_port
    );

    // If headless server mode requested or DISPLAY is unset
    let has_display = env::var("DISPLAY").is_ok() || env::var("WAYLAND_DISPLAY").is_ok();
    if args.server || !has_display {
        println!(
            "Running in server mode. Open http://127.0.0.1:{} in your browser.",
            actual_port
        );
        rt.block_on(async {
            let _ = server_handle.await;
        });
        return Ok(());
    }

    // Launch desktop native window via wry + tao
    #[cfg(not(target_os = "android"))]
    {
        use tao::dpi::LogicalSize;
        use tao::event::{Event, StartCause, WindowEvent};
        use tao::event_loop::{ControlFlow, EventLoop};
        use tao::window::WindowBuilder;
        use wry::WebViewBuilder;

        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_title("Gridloom — SQL-Connected Worksheet")
            .with_inner_size(LogicalSize::new(1280.0, 800.0))
            .build(&event_loop)?;

        let webview_url = format!("http://127.0.0.1:{}", actual_port);
        let _webview = WebViewBuilder::new()
            .with_url(&webview_url)
            .build(&window)?;

        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;
            match event {
                Event::NewEvents(StartCause::Init) => println!("Gridloom desktop window opened."),
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => *control_flow = ControlFlow::Exit,
                _ => (),
            }
        });
    }

    #[cfg(target_os = "android")]
    {
        rt.block_on(async {
            let _ = server_handle.await;
        });
    }

    Ok(())
}
