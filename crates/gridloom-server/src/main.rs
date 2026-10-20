use clap::Parser;
use gridloom_server::benchmark::run_performance_benchmarks;
use gridloom_server::start_server;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(name = "gridloom-server", about = "Gridloom local desktop server")]
struct Args {
    #[arg(short, long, default_value_t = 3001)]
    port: u16,

    #[arg(long, default_value_t = false)]
    bench: bool,

    #[arg(long, default_value_t = false)]
    no_samples: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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

    let (port, handle) = start_server(args.port, !args.no_samples).await?;
    println!("Gridloom Server running at http://127.0.0.1:{}", port);
    println!("Press Ctrl+C to terminate.");
    handle.await?;
    Ok(())
}
