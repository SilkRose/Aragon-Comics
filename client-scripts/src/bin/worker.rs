use aragon_comics_client_scripts::Worker;
use gloo_worker::Registrable as _;

fn main() {
	std::panic::set_hook(Box::new(console_error_panic_hook::hook));

	Worker::registrar().register();
}
