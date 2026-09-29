use aragon_comics_client_scripts::Worker;
use gloo_worker::Spawnable as _;

fn main() {
	std::panic::set_hook(Box::new(console_error_panic_hook::hook));
	let worker = Worker::spawner().spawn("/worker.js");

	// let fish = worker.run(0.0).await;
}
