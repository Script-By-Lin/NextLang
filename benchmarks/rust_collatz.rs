fn main() {
    let start = std::time::Instant::now();
    let mut total_steps: i64 = 0;
    for n in 1..=50000i64 {
        let mut cur = n;
        while cur > 1 {
            if cur % 2 == 0 {
                cur = cur / 2;
            } else {
                cur = cur * 3 + 1;
            }
            total_steps += 1;
        }
    }
    let elapsed = start.elapsed();
    println!("Collatz total steps: {}", total_steps);
    eprintln!("Rust Execution time: {:?}", elapsed);
}
