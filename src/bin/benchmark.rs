use num_cpus;
use std::time::Instant;

fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║              🔬 THREAD PERFORMANCE BENCHMARK                 ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    
    let max_threads = num_cpus::get();
    println!("Detected {} CPU cores\n", max_threads);
    
    println!("Testing different thread counts...\n");
    
    for threads in 1..=max_threads {
        println!("Testing {} thread(s)...", threads);
        
        let start = Instant::now();
        
        // Simulate mining work
        let handles: Vec<_> = (0..threads)
            .map(|_| {
                std::thread::spawn(|| {
                    let mut sum = 0u64;
                    for i in 0..10_000_000 {
                        sum = sum.wrapping_add(i);
                    }
                    sum
                })
            })
            .collect();
        
        for handle in handles {
            handle.join().unwrap();
        }
        
        let elapsed = start.elapsed();
        let rate = 10_000_000.0 * threads as f64 / elapsed.as_secs_f64();
        
        println!("  Time: {:.3}s | Rate: {:.0} ops/s\n", 
                 elapsed.as_secs_f64(), rate);
    }
    
    println!("✅ Benchmark complete!");
}