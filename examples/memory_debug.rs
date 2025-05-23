use sysinfo::{System, ProcessesToUpdate};

fn main() {
    let mut system = System::new_all();
    
    println!("=== Process Memory Debug ===");
    println!("Total processes: {}\n", system.processes().len());
    
    // Look for llama processes
    println!("Llama-related processes:");
    for (pid, process) in system.processes() {
        let name = process.name().to_string_lossy();
        if name.contains("llama") {
            let cmd_vec: Vec<String> = process.cmd().iter()
                .map(|s| s.to_string_lossy().into_owned())
                .collect();
            let cmd = cmd_vec.join(" ");
            
            println!("PID: {}", pid);
            println!("  Name: {}", name);
            println!("  Memory: {} KB ({:.2} MB)", process.memory(), process.memory() as f64 / 1024.0);
            println!("  Parent PID: {:?}", process.parent());
            println!("  CMD: {}", if cmd.len() > 100 { &cmd[..100] } else { &cmd });
            println!();
        }
    }
    
    // Also check for high memory processes
    println!("\nTop 5 memory consumers:");
    let mut processes: Vec<_> = system.processes().iter().collect();
    processes.sort_by(|a, b| b.1.memory().cmp(&a.1.memory()));
    
    for (i, (pid, process)) in processes.iter().take(5).enumerate() {
        println!("{}. {} (PID: {}): {:.2} GB", 
            i + 1,
            process.name().to_string_lossy(),
            pid,
            process.memory() as f64 / 1024.0 / 1024.0
        );
    }
}