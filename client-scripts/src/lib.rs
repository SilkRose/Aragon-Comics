use gloo_worker::oneshot::oneshot;

#[oneshot]
pub async fn Worker(buf: i32) -> i32 {
	// do stuff here
	buf
}
