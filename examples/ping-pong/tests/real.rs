#[test]
fn basic() {
    let pinger = std::thread::spawn(pingpong::run::pinger);
    let ponger = std::thread::spawn(pingpong::run::ponger);

    pinger.join().unwrap();
    ponger.join().unwrap();
}
