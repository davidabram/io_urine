use io_urine::IoUring;

fn main() {
    let mut ring = IoUring::new(8).expect("Failed to create io_uring");

    println!("io_urine example - Hello, io_uring!");
    println!("Submission queue space: {}", ring.sq_space_left());
    println!("Completion queue empty: {}", ring.is_cq_empty());

    let _sqe = ring.nop().expect("Failed to get SQE");
    println!("Prepared NOP operation");

    let submitted = ring.submit().expect("Failed to submit");
    println!("Submitted {} operations", submitted);

    let _wait_result = ring.submit_and_wait(0).expect("Failed to wait");
    println!("Waited for 1 completion");

    if !ring.is_cq_empty() {
        ring.copy_cqes(1);
    }

    println!("Example completed successfully!");
}
